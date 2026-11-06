use gtk::glib;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FastFlagValue {
    Toggle(bool),
    Number(i64),
}

#[derive(Debug, Default, Clone, PartialEq, Eq, glib::Boxed, Serialize, Deserialize)]
#[boxed_type(name = "QuadraFastFlags")]
#[serde(transparent)]
pub struct FastFlags(pub BTreeMap<String, FastFlagValue>);

impl FastFlags {
    pub fn get(&self, name: &str) -> Option<FastFlagValue> {
        self.0.get(name).copied()
    }

    pub fn with(mut self, name: &str, value: Option<FastFlagValue>) -> Self {
        match value {
            Some(value) => self.0.insert(name.to_owned(), value),
            None => self.0.remove(name),
        };
        self
    }
}
