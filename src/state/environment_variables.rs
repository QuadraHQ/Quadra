use gtk::glib;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvironmentVariable {
    pub name: String,
    pub value: String,
    pub enabled: bool,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, glib::Boxed, Serialize, Deserialize)]
#[boxed_type(name = "QuadraEnvironmentVariables")]
#[serde(transparent)]
pub struct EnvironmentVariables(pub Vec<EnvironmentVariable>);

impl EnvironmentVariables {
    pub fn enabled(&self) -> impl Iterator<Item = &EnvironmentVariable> {
        self.0
            .iter()
            .filter(|variable| variable.enabled && !variable.name.is_empty())
    }
}
