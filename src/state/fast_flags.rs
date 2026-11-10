use gtk::glib;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FastFlagValue {
    Toggle(bool),
    Number(i64),
    Text(String),
}

impl FastFlagValue {
    pub fn parse(text: &str) -> Self {
        let text = text.trim();
        if text.eq_ignore_ascii_case("true") {
            Self::Toggle(true)
        } else if text.eq_ignore_ascii_case("false") {
            Self::Toggle(false)
        } else if let Ok(number) = text.parse() {
            Self::Number(number)
        } else {
            Self::Text(text.to_owned())
        }
    }

    pub fn as_text(&self) -> String {
        match self {
            Self::Toggle(true) => "true".to_owned(),
            Self::Toggle(false) => "false".to_owned(),
            Self::Number(number) => number.to_string(),
            Self::Text(text) => text.clone(),
        }
    }

    fn from_json(value: &Value) -> Option<Self> {
        match value {
            Value::Bool(toggle) => Some(Self::Toggle(*toggle)),
            Value::Number(number) => number.as_i64().map(Self::Number),
            Value::String(text) => Some(Self::parse(text)),
            _ => None,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, glib::Boxed, Serialize, Deserialize)]
#[boxed_type(name = "QuadraFastFlags")]
#[serde(transparent)]
pub struct FastFlags(pub BTreeMap<String, FastFlagValue>);

impl FastFlags {
    pub fn get(&self, name: &str) -> Option<FastFlagValue> {
        self.0.get(name).cloned()
    }

    pub fn with(mut self, name: &str, value: Option<FastFlagValue>) -> Self {
        match value {
            Some(value) => self.0.insert(name.to_owned(), value),
            None => self.0.remove(name),
        };
        self
    }

    pub fn merged_with_json(mut self, json: &str) -> Result<(Self, usize), String> {
        let Value::Object(imported) = serde_json::from_str(json).map_err(|error| error.to_string())?
        else {
            return Err("The text isn't a JSON object of flags.".to_owned());
        };

        let mut count = 0;
        for (name, value) in imported {
            if let Some(value) = FastFlagValue::from_json(&value) {
                self.0.insert(name, value);
                count += 1;
            }
        }
        Ok((self, count))
    }

    pub fn as_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_what_the_user_types() {
        assert_eq!(FastFlagValue::parse(" True "), FastFlagValue::Toggle(true));
        assert_eq!(FastFlagValue::parse("-1"), FastFlagValue::Number(-1));
        assert_eq!(
            FastFlagValue::parse("none"),
            FastFlagValue::Text("none".to_owned())
        );
    }

    #[test]
    fn imports_native_and_stringly_typed_flags() {
        let existing = FastFlags::default().with("Keep", Some(FastFlagValue::Number(1)));
        let (flags, count) = existing
            .merged_with_json(r#"{"A": true, "B": "False", "C": 7, "D": "x", "E": null, "F": 1.5}"#)
            .unwrap();

        assert_eq!(count, 4);
        assert_eq!(flags.get("Keep"), Some(FastFlagValue::Number(1)));
        assert_eq!(flags.get("A"), Some(FastFlagValue::Toggle(true)));
        assert_eq!(flags.get("B"), Some(FastFlagValue::Toggle(false)));
        assert_eq!(flags.get("C"), Some(FastFlagValue::Number(7)));
        assert_eq!(flags.get("D"), Some(FastFlagValue::Text("x".to_owned())));
        assert_eq!(flags.get("E"), None);
    }

    #[test]
    fn rejects_text_that_is_not_an_object() {
        assert!(FastFlags::default().merged_with_json("[1]").is_err());
        assert!(FastFlags::default().merged_with_json("nope").is_err());
    }
}
