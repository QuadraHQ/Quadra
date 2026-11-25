use crate::connection::Reply;
use crate::protocol::{self, Script};
use gtk::glib::translate::ToGlibPtr;
use gtk::{gio, glib};
use serde_json::{Value, json};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;
use std::time::Duration;
use webkit::prelude::*;
use webkit::{
    NavigationPolicyDecision, NetworkSession, PolicyDecision, PolicyDecisionType,
    ResponsePolicyDecision, URIRequest, UserContentInjectedFrames, UserContentManager, UserScript,
    UserScriptInjectionTime, WebView,
};

const DECISION_TIMEOUT: Duration = Duration::from_secs(15);
const SCHEMES_THE_WINDOW_OPENS: [&str; 5] = ["https://", "http://", "about:", "data:", "blob:"];

#[derive(Clone, Default)]
pub struct Decisions(Rc<RefCell<DecisionList>>);

#[derive(Default)]
struct DecisionList {
    last: u64,
    waiting: HashMap<u64, Waiting>,
}

struct Waiting {
    decision: PolicyDecision,
    url: String,
}

impl Decisions {
    fn wait_for(&self, decision: &PolicyDecision, url: &str) -> u64 {
        let number = {
            let mut list = self.0.borrow_mut();
            list.last += 1;
            let number = list.last;
            list.waiting.insert(
                number,
                Waiting {
                    decision: decision.clone(),
                    url: url.to_owned(),
                },
            );
            number
        };
        let decisions = self.clone();
        glib::timeout_add_local_once(DECISION_TIMEOUT, move || decisions.resolve(number, false));
        number
    }

    pub fn resolve(&self, number: u64, allow: bool) {
        let waiting = self.0.borrow_mut().waiting.remove(&number);
        if let Some(waiting) = waiting {
            if allow && window_can_open(&waiting.url) {
                waiting.decision.use_();
            } else {
                waiting.decision.ignore();
            }
        }
    }

    pub fn refuse_all(&self) {
        let waiting: Vec<_> = self.0.borrow_mut().waiting.drain().collect();
        for (_, waiting) in waiting {
            waiting.decision.ignore();
        }
    }
}

fn window_can_open(url: &str) -> bool {
    SCHEMES_THE_WINDOW_OPENS
        .iter()
        .any(|scheme| url.starts_with(scheme))
}

pub struct Settings {
    pub user_agent: String,
    pub developer_extras: bool,
    pub zoom: f64,
    pub tab_focuses_links: bool,
}

pub struct Page {
    pub view: WebView,
    handlers: Vec<String>,
    view_number: u64,
    reply: Reply,
}

impl Page {
    pub fn new(
        view_number: u64,
        session: &NetworkSession,
        decisions: &Decisions,
        reply: Reply,
    ) -> Self {
        let view = WebView::builder()
            .network_session(session)
            .user_content_manager(&UserContentManager::new())
            .hexpand(true)
            .vexpand(true)
            .build();
        report_loads(&view, view_number, &reply);
        ask_before_navigating(&view, view_number, decisions, &reply);

        Self {
            view,
            handlers: Vec::new(),
            view_number,
            reply,
        }
    }

    pub fn configure(&mut self, settings: Settings, scripts: &[Script], handlers: Vec<String>) {
        self.view.set_zoom_level(settings.zoom);
        if let Some(page) = WebViewExt::settings(&self.view) {
            page.set_user_agent(Some(&settings.user_agent));
            page.set_enable_developer_extras(settings.developer_extras);
            page.set_enable_tabs_to_links(settings.tab_focuses_links);
        }
        let Some(manager) = self.view.user_content_manager() else {
            return;
        };
        manager.remove_all_scripts();
        for script in scripts {
            manager.add_script(&user_script(script));
        }
        for removed in self.handlers.iter().filter(|old| !handlers.contains(old)) {
            manager.unregister_script_message_handler(removed, None);
        }
        for added in handlers.iter().filter(|new| !self.handlers.contains(new)) {
            self.listen_for_messages(&manager, added);
        }
        self.handlers = handlers;
    }

    fn listen_for_messages(&self, manager: &UserContentManager, name: &str) {
        let view_number = self.view_number;
        let reply = self.reply.clone();
        let view = self.view.downgrade();
        let handler = name.to_owned();
        if !manager.register_script_message_handler(name, None) {
            return;
        }
        manager.connect_script_message_received(Some(name), move |_, value| {
            let Some(view) = view.upgrade() else {
                return;
            };
            let body = value
                .to_json(0)
                .and_then(|json| serde_json::from_str::<Value>(&json).ok());
            reply.send(about(
                &view,
                json!({
                    "event": "message",
                    "view": view_number,
                    "name": handler,
                    "body": body,
                }),
            ));
        });
    }

    pub fn load(&self, url: &str, headers: &BTreeMap<String, String>) {
        let request = URIRequest::new(url);
        if let Some(fields) = request.http_headers() {
            for (name, value) in headers {
                if is_safe_header(name, value) {
                    fields.replace(name, value);
                }
            }
        }
        self.view.load_request(&request);
    }

    pub fn evaluate(&self, script: &str, request: Option<u64>) {
        let reply = self.reply.clone();
        self.view
            .evaluate_javascript(script, None, None, gio::Cancellable::NONE, move |result| {
                let Some(request) = request else {
                    return;
                };
                let value = result
                    .map(|value| {
                        value
                            .to_json(0)
                            .and_then(|json| serde_json::from_str(&json).ok())
                            .unwrap_or(Value::Null)
                    })
                    .map_err(|error| error.message().to_owned());
                reply.send(protocol::reply(request, value));
            });
    }
}

fn is_safe_header(name: &str, value: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|character| character.is_ascii_graphic() && character != ':')
        && !value
            .chars()
            .any(|character| matches!(character, '\r' | '\n' | '\0'))
}

fn user_script(script: &Script) -> UserScript {
    let frames = if script.main_only {
        UserContentInjectedFrames::TopFrame
    } else {
        UserContentInjectedFrames::AllFrames
    };
    let time = if script.at_start {
        UserScriptInjectionTime::Start
    } else {
        UserScriptInjectionTime::End
    };
    UserScript::new(&script.source, frames, time, &[], &[])
}

fn about(view: &WebView, mut event: Value) -> Value {
    event["currentUrl"] = json!(view.uri().map(|uri| uri.to_string()).unwrap_or_default());
    event["back"] = json!(view.can_go_back());
    event["forward"] = json!(view.can_go_forward());
    event
}

fn report_loads(view: &WebView, view_number: u64, reply: &Reply) {
    let sender = reply.clone();
    view.connect_load_changed(move |view, stage| {
        let stage = glib::translate::IntoGlib::into_glib(stage);
        sender.send(about(
            view,
            json!({ "event": "load", "view": view_number, "stage": stage }),
        ));
    });

    let sender = reply.clone();
    view.connect_uri_notify(move |view| {
        sender.send(about(
            view,
            json!({ "event": "state", "view": view_number }),
        ));
    });

    let sender = reply.clone();
    view.connect_load_failed(move |view, stage, _, error| {
        let (domain, code) = domain_and_code(error);
        eprintln!("quadra-web-window: a page failed to load ({domain} {code})");
        sender.send(about(
            view,
            json!({
                "event": "failed",
                "view": view_number,
                "provisional": matches!(
                    stage,
                    webkit::LoadEvent::Started | webkit::LoadEvent::Redirected
                ),
                "domain": domain,
                "code": code,
                "message": error.message(),
            }),
        ));
        true
    });

    let sender = reply.clone();
    view.connect_web_process_terminated(move |view, reason| {
        eprintln!("quadra-web-window: the web process stopped ({reason:?})");
        sender.send(about(
            view,
            json!({ "event": "terminated", "view": view_number }),
        ));
    });
}

fn domain_and_code(error: &glib::Error) -> (String, i32) {
    let raw: *const glib::ffi::GError = error.to_glib_none().0;
    let (domain, code) = unsafe { ((*raw).domain, (*raw).code) };
    let domain: glib::Quark = unsafe { glib::translate::from_glib(domain) };
    (domain.as_str().to_string(), code)
}

fn ask_before_navigating(view: &WebView, view_number: u64, decisions: &Decisions, reply: &Reply) {
    let sender = reply.clone();
    let decisions = decisions.clone();
    view.connect_decide_policy(move |view, decision, kind| {
        let Some(mut event) = describe(decision, kind) else {
            return false;
        };
        let url = event["url"].as_str().unwrap_or_default().to_owned();
        event["view"] = json!(view_number);
        event["decision"] = json!(decisions.wait_for(decision, &url));
        sender.send(about(view, event));
        true
    });

    let sender = reply.clone();
    view.connect_create(move |view, action| {
        let url = action
            .clone()
            .request()
            .and_then(|request| request.uri())
            .map(|uri| uri.to_string())
            .unwrap_or_default();
        sender.send(about(
            view,
            json!({ "event": "new-window", "view": view_number, "url": url }),
        ));
        None
    });
}

fn describe(decision: &PolicyDecision, kind: PolicyDecisionType) -> Option<Value> {
    match kind {
        PolicyDecisionType::NavigationAction | PolicyDecisionType::NewWindowAction => {
            let action = decision
                .downcast_ref::<NavigationPolicyDecision>()?
                .navigation_action()?;
            let url = action.request()?.uri()?.to_string();
            let navigation_type = glib::translate::IntoGlib::into_glib(action.navigation_type());
            Some(json!({
                "event": "navigation",
                "url": url,
                "type": navigation_type,
                "newWindow": kind == PolicyDecisionType::NewWindowAction,
            }))
        }
        PolicyDecisionType::Response => {
            let decision = decision.downcast_ref::<ResponsePolicyDecision>()?;
            let response = decision.response()?;
            Some(json!({
                "event": "response",
                "url": response.uri()?.to_string(),
                "status": response.status_code(),
                "main": decision.is_main_frame_main_resource(),
            }))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_web_addresses_open_in_the_window() {
        assert!(window_can_open("https://www.roblox.com/login"));
        assert!(window_can_open("about:blank"));
        assert!(!window_can_open("file:///etc/passwd"));
        assert!(!window_can_open("roblox://placeId=1"));
    }

    #[test]
    fn headers_that_would_split_a_request_are_refused() {
        assert!(is_safe_header("X-CSRF-TOKEN", "abc"));
        assert!(!is_safe_header("X-Test", "a\r\nCookie: b"));
        assert!(!is_safe_header("Bad Name", "a"));
        assert!(!is_safe_header("", "a"));
    }
}
