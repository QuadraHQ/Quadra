use crate::connection::Reply;
use crate::protocol::{self, Cookie};
use gtk::{gio, glib};
use serde_json::{Value, json};
use webkit::{CookieManager, CookiePersistentStorage, NetworkSession};

const SESSION_COOKIE: i32 = -1;
const COOKIE_FILE: &str = "cookies.sqlite";

pub fn keep_between_launches(session: &NetworkSession, data_directory: &std::path::Path) {
    if let (Some(manager), Some(file)) = (
        session.cookie_manager(),
        data_directory.join(COOKIE_FILE).to_str(),
    ) {
        manager.set_persistent_storage(file, CookiePersistentStorage::Sqlite);
    }
}

pub fn send_all(session: &NetworkSession, request: u64, reply: Reply) {
    let Some(manager) = session.cookie_manager() else {
        reply.send(protocol::reply(request, Err(no_cookie_store())));
        return;
    };
    manager.all_cookies(gio::Cancellable::NONE, move |cookies| {
        let cookies = cookies
            .map(|cookies| cookies.into_iter().map(described).collect::<Vec<_>>())
            .map(Value::from)
            .map_err(|error| error.to_string());
        reply.send(protocol::reply(request, cookies));
    });
}

pub enum Change {
    Set,
    Delete,
}

pub fn change(
    session: &NetworkSession,
    change: Change,
    cookie: Cookie,
    request: Option<u64>,
    reply: Reply,
) {
    let answer = move |result: Result<(), glib::Error>| {
        if let Some(request) = request {
            let result = result
                .map(|()| Value::Null)
                .map_err(|error| error.to_string());
            reply.send(protocol::reply(request, result));
        }
    };
    let Some(manager) = session.cookie_manager() else {
        answer(Err(glib::Error::new(
            gio::IOErrorEnum::NotSupported,
            &no_cookie_store(),
        )));
        return;
    };
    apply(&manager, change, &soup_cookie(&cookie), answer);
}

fn apply(
    manager: &CookieManager,
    change: Change,
    cookie: &soup::Cookie,
    answer: impl FnOnce(Result<(), glib::Error>) + 'static,
) {
    match change {
        Change::Set => manager.add_cookie(cookie, gio::Cancellable::NONE, answer),
        Change::Delete => manager.delete_cookie(cookie, gio::Cancellable::NONE, answer),
    }
}

fn no_cookie_store() -> String {
    "There is no cookie store.".to_owned()
}

fn soup_cookie(cookie: &Cookie) -> soup::Cookie {
    let mut soup_cookie = soup::Cookie::new(
        &cookie.name,
        &cookie.value,
        &cookie.domain,
        &cookie.path,
        SESSION_COOKIE,
    );
    soup_cookie.set_secure(cookie.secure);
    if let Some(expires) = cookie.expires
        && let Ok(expires) = glib::DateTime::from_unix_utc(expires)
    {
        soup_cookie.set_expires(&expires);
    }
    soup_cookie
}

fn described(mut cookie: soup::Cookie) -> Value {
    json!({
        "name": cookie.name().map(|name| name.to_string()),
        "value": cookie.value().map(|value| value.to_string()),
        "domain": cookie.domain().map(|domain| domain.to_string()),
        "path": cookie.path().map(|path| path.to_string()),
        "secure": cookie.is_secure(),
        "expires": cookie.expires().map(|expires| expires.to_unix()),
    })
}
