use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "kebab-case", rename_all_fields = "camelCase")]
pub enum Request {
    Configure {
        view: u64,
        user_agent: String,
        developer_extras: bool,
        zoom: f64,
        tab_focuses_links: bool,
        scripts: Vec<Script>,
        handlers: Vec<String>,
    },
    Load {
        view: u64,
        url: String,
        headers: BTreeMap<String, String>,
    },
    Eval {
        view: u64,
        script: String,
        request: Option<u64>,
    },
    Back {
        view: u64,
    },
    Forward {
        view: u64,
    },
    Stop {
        view: u64,
    },
    Close {
        view: u64,
    },
    Visible {
        view: u64,
        visible: bool,
    },
    Policy {
        decision: u64,
        allow: bool,
    },
    CookiesGet {
        request: u64,
    },
    CookieSet {
        cookie: Cookie,
        request: Option<u64>,
    },
    CookieDelete {
        cookie: Cookie,
        request: Option<u64>,
    },
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Script {
    pub source: String,
    pub at_start: bool,
    pub main_only: bool,
}

#[derive(Debug, Deserialize)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub secure: bool,
    pub expires: Option<i64>,
}

pub fn reply(request: u64, result: Result<Value, String>) -> Value {
    match result {
        Ok(value) => json!({ "event": "reply", "request": request, "value": value }),
        Err(error) => json!({ "event": "reply", "request": request, "error": error }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(line: &str) -> Request {
        serde_json::from_str(line).unwrap()
    }

    #[test]
    fn reads_the_requests_rosemary_sends() {
        let configure = parse(
            r#"{"op":"configure","view":1,"userAgent":"a","developerExtras":false,
                "zoom":1.0,"tabFocusesLinks":false,
                "scripts":[{"source":"s","atStart":true,"mainOnly":false}],"handlers":["h"]}"#,
        );
        assert!(matches!(
            configure,
            Request::Configure { view: 1, ref scripts, .. } if scripts[0].at_start
        ));
        assert!(matches!(
            parse(r#"{"op":"cookies-get","request":4}"#),
            Request::CookiesGet { request: 4 }
        ));
        assert!(matches!(
            parse(r#"{"op":"eval","view":2,"script":"1"}"#),
            Request::Eval { request: None, .. }
        ));
        assert!(matches!(
            parse(
                r#"{"op":"cookie-set","request":5,"cookie":{"name":"n","value":"v",
                    "domain":".example.com","path":"/","secure":true,"expires":null}}"#
            ),
            Request::CookieSet {
                request: Some(5),
                ..
            }
        ));
    }

    #[test]
    fn a_failed_reply_carries_the_reason_and_no_value() {
        let reply = reply(3, Err("broken".to_owned()));

        assert_eq!(reply["request"], 3);
        assert_eq!(reply["error"], "broken");
        assert!(reply.get("value").is_none());
    }
}
