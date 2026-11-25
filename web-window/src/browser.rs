use crate::connection::Reply;
use crate::cookies::{self, Change};
use crate::page::{Decisions, Page, Settings};
use crate::protocol::Request;
use adw::prelude::*;
use anyhow::Context;
use gtk::glib;
use serde_json::json;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fs::DirBuilder;
use std::os::unix::fs::DirBuilderExt;
use std::path::Path;
use std::rc::Rc;
use webkit::NetworkSession;
use webkit::prelude::*;

const OWNER_ONLY: u32 = 0o700;
const WINDOW_TITLE: &str = "Roblox";
const DEFAULT_WIDTH: i32 = 1100;
const DEFAULT_HEIGHT: i32 = 800;

#[derive(Clone)]
pub struct Browser(Rc<Inner>);

struct Inner {
    session: NetworkSession,
    decisions: Decisions,
    state: RefCell<State>,
    window: Window,
}

#[derive(Default)]
struct State {
    reply: Option<Reply>,
    pages: HashMap<u64, Page>,
    on_screen: Vec<u64>,
    dismissed: HashSet<u64>,
}

struct Window {
    window: adw::Window,
    title: adw::WindowTitle,
    back: gtk::Button,
    forward: gtk::Button,
    reload: gtk::Button,
    done: gtk::Button,
    stack: gtk::Stack,
}

impl Browser {
    pub fn new(data_directory: &Path) -> anyhow::Result<Self> {
        DirBuilder::new()
            .recursive(true)
            .mode(OWNER_ONLY)
            .create(data_directory)
            .with_context(|| format!("Couldn't create {}", data_directory.display()))?;
        let cache_directory = data_directory.join("cache");
        let session = NetworkSession::new(data_directory.to_str(), cache_directory.to_str());
        cookies::keep_between_launches(&session, data_directory);

        let browser = Self(Rc::new(Inner {
            session,
            decisions: Decisions::default(),
            state: RefCell::default(),
            window: Window::new(),
        }));
        browser.connect_window();

        Ok(browser)
    }

    pub fn connected(&self, reply: Reply) {
        self.0.state.borrow_mut().reply = Some(reply);
    }

    pub fn disconnected(&self) {
        self.0.decisions.refuse_all();
        let pages = std::mem::take(&mut *self.0.state.borrow_mut()).pages;
        for page in pages.into_values() {
            self.discard(page);
        }
        self.0.window.window.set_visible(false);
    }

    pub fn handle(&self, request: Request) {
        let Some(reply) = self.0.state.borrow().reply.clone() else {
            return;
        };
        match request {
            Request::Configure {
                view,
                user_agent,
                developer_extras,
                zoom,
                tab_focuses_links,
                scripts,
                handlers,
            } => self.with_page(view, |page| {
                let settings = Settings {
                    user_agent,
                    developer_extras,
                    zoom,
                    tab_focuses_links,
                };
                page.configure(settings, &scripts, handlers);
            }),
            Request::Load { view, url, headers } => {
                self.0.state.borrow_mut().dismissed.remove(&view);
                self.with_page(view, |page| page.load(&url, &headers));
                self.bring_forward(view);
            }
            Request::Eval {
                view,
                script,
                request,
            } => self.with_page(view, |page| page.evaluate(&script, request)),
            Request::Back { view } => self.with_page(view, |page| page.view.go_back()),
            Request::Forward { view } => self.with_page(view, |page| page.view.go_forward()),
            Request::Stop { view } => self.with_page(view, |page| page.view.stop_loading()),
            Request::Close { view } => self.close(view),
            Request::Visible { view, visible } => self.set_on_screen(view, visible),
            Request::Policy { decision, allow } => self.0.decisions.resolve(decision, allow),
            Request::CookiesGet { request } => {
                cookies::send_all(&self.0.session, request, reply);
            }
            Request::CookieSet { cookie, request } => {
                cookies::change(&self.0.session, Change::Set, cookie, request, reply);
            }
            Request::CookieDelete { cookie, request } => {
                cookies::change(&self.0.session, Change::Delete, cookie, request, reply);
            }
        }
    }

    fn with_page(&self, view: u64, body: impl FnOnce(&mut Page)) {
        let Some(mut page) = self.take_or_create(view) else {
            return;
        };
        body(&mut page);
        self.0.state.borrow_mut().pages.insert(view, page);
        self.show_the_current_page();
    }

    fn take_or_create(&self, view: u64) -> Option<Page> {
        if let Some(page) = self.0.state.borrow_mut().pages.remove(&view) {
            return Some(page);
        }
        let reply = self.0.state.borrow().reply.clone()?;
        let page = Page::new(view, &self.0.session, &self.0.decisions, reply);
        self.0.window.stack.add_child(&page.view);
        let browser = Rc::downgrade(&self.0);
        let follow = move || {
            if let Some(inner) = browser.upgrade() {
                let browser = Browser(inner);
                glib::idle_add_local_once(move || browser.update_header());
            }
        };
        let on_title = follow.clone();
        page.view.connect_title_notify(move |_| on_title());
        page.view.connect_uri_notify(move |_| follow());
        Some(page)
    }

    fn close(&self, view: u64) {
        let page = {
            let mut state = self.0.state.borrow_mut();
            state.on_screen.retain(|shown| *shown != view);
            state.pages.remove(&view)
        };
        if let Some(page) = page {
            self.discard(page);
        }
        self.show_the_current_page();
    }

    fn discard(&self, page: Page) {
        page.view.stop_loading();
        self.0.window.stack.remove(&page.view);
    }

    fn set_on_screen(&self, view: u64, visible: bool) {
        {
            let mut state = self.0.state.borrow_mut();
            state.on_screen.retain(|shown| *shown != view);
            state.dismissed.remove(&view);
            if visible {
                state.on_screen.push(view);
            }
        }
        self.show_the_current_page();
        if visible {
            self.0.window.window.present();
        }
    }

    fn bring_forward(&self, view: u64) {
        let is_on_screen = self.0.state.borrow().on_screen.contains(&view);
        if is_on_screen {
            self.set_on_screen(view, true);
        }
    }

    fn current_view(&self) -> Option<(u64, webkit::WebView)> {
        let state = self.0.state.borrow();
        state
            .on_screen
            .iter()
            .rev()
            .filter(|number| !state.dismissed.contains(number))
            .find_map(|number| {
                state
                    .pages
                    .get(number)
                    .map(|page| (*number, page.view.clone()))
            })
    }

    fn show_the_current_page(&self) {
        match self.current_view() {
            Some((_, view)) => {
                self.0.window.stack.set_visible_child(&view);
                if !self.0.window.window.is_visible() {
                    self.0.window.window.present();
                    view.grab_focus();
                }
            }
            None => self.0.window.window.set_visible(false),
        }
        self.update_header();
    }

    fn update_header(&self) {
        let window = &self.0.window;
        let view = self.current_view().map(|(_, view)| view);
        let title = view
            .as_ref()
            .and_then(|view| view.title())
            .filter(|title| !title.is_empty());
        let host = view
            .as_ref()
            .and_then(|view| view.uri())
            .and_then(|uri| host_of(&uri).map(str::to_owned));
        window
            .title
            .set_title(title.as_deref().unwrap_or(WINDOW_TITLE));
        window
            .title
            .set_subtitle(host.as_deref().unwrap_or_default());
        window
            .back
            .set_sensitive(view.as_ref().is_some_and(|view| view.can_go_back()));
        window
            .forward
            .set_sensitive(view.as_ref().is_some_and(|view| view.can_go_forward()));
    }

    fn dismiss(&self) {
        if let Some((number, _)) = self.current_view() {
            let reply = {
                let mut state = self.0.state.borrow_mut();
                state.dismissed.insert(number);
                state.reply.clone()
            };
            if let Some(reply) = reply {
                reply.send(json!({ "event": "closed", "view": number }));
            }
        }
        self.show_the_current_page();
    }

    fn connect_window(&self) {
        let window = &self.0.window;
        let browser = Rc::downgrade(&self.0);
        let act = move |action: fn(&Browser)| {
            if let Some(inner) = browser.upgrade() {
                action(&Browser(inner));
            }
        };

        let on_close = act.clone();
        window.window.connect_close_request(move |_| {
            on_close(Browser::dismiss);
            glib::Propagation::Stop
        });
        let on_done = act.clone();
        window
            .done
            .connect_clicked(move |_| on_done(Browser::dismiss));
        let on_back = act.clone();
        window.back.connect_clicked(move |_| {
            on_back(|browser| {
                if let Some((_, view)) = browser.current_view() {
                    view.go_back();
                }
            });
        });
        let on_forward = act.clone();
        window.forward.connect_clicked(move |_| {
            on_forward(|browser| {
                if let Some((_, view)) = browser.current_view() {
                    view.go_forward();
                }
            });
        });
        window.reload.connect_clicked(move |_| {
            act(|browser| {
                if let Some((_, view)) = browser.current_view() {
                    view.reload();
                }
            });
        });
    }
}

impl Window {
    fn new() -> Self {
        let window = adw::Window::builder()
            .title(WINDOW_TITLE)
            .default_width(DEFAULT_WIDTH)
            .default_height(DEFAULT_HEIGHT)
            .build();
        let title = adw::WindowTitle::new(WINDOW_TITLE, "");
        let header = adw::HeaderBar::builder().title_widget(&title).build();
        let back = header_button("go-previous-symbolic", "Back");
        let forward = header_button("go-next-symbolic", "Forward");
        let reload = header_button("view-refresh-symbolic", "Reload");
        for button in [&back, &forward, &reload] {
            header.pack_start(button);
        }
        let done = gtk::Button::builder()
            .label("Back to Roblox")
            .css_classes(["suggested-action"])
            .build();
        header.pack_end(&done);

        let stack = gtk::Stack::new();
        let layout = adw::ToolbarView::builder().content(&stack).build();
        layout.add_top_bar(&header);
        window.set_content(Some(&layout));

        Self {
            window,
            title,
            back,
            forward,
            reload,
            done,
            stack,
        }
    }
}

fn header_button(icon: &str, tooltip: &str) -> gtk::Button {
    gtk::Button::builder()
        .icon_name(icon)
        .tooltip_text(tooltip)
        .build()
}

fn host_of(url: &str) -> Option<&str> {
    let (_, rest) = url.split_once("://")?;
    let authority = rest.split(['/', '?', '#']).next()?;
    let host = authority.rsplit('@').next()?;
    (!host.is_empty()).then_some(host)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_header_shows_the_host_that_is_really_loaded() {
        assert_eq!(
            host_of("https://www.roblox.com@evil.example/login?x=1"),
            Some("evil.example")
        );
        assert_eq!(host_of("about:blank"), None);
    }
}
