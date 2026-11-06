use super::roblox;
use crate::state::{FastFlagValue, FastFlags};
use anyhow::Context;
use std::collections::BTreeMap;
use std::fs;

const CLIENT_SETTINGS_INSIDE_APP: &str = "Contents/MacOS/ClientSettings/ClientAppSettings.json";

pub fn write_client_settings(flags: &FastFlags) -> anyhow::Result<()> {
    let path = roblox::app_path().join(CLIENT_SETTINGS_INSIDE_APP);
    let client_settings: BTreeMap<&str, String> = flags
        .0
        .iter()
        .map(|(name, &value)| (name.as_str(), client_setting_value(value)))
        .collect();

    if let Some(directory) = path.parent() {
        fs::create_dir_all(directory)?;
    }
    fs::write(&path, serde_json::to_string_pretty(&client_settings)?)
        .with_context(|| format!("Couldn't write the FastFlags to {}", path.display()))
}

fn client_setting_value(value: FastFlagValue) -> String {
    match value {
        FastFlagValue::Toggle(true) => "True".to_owned(),
        FastFlagValue::Toggle(false) => "False".to_owned(),
        FastFlagValue::Number(number) => number.to_string(),
    }
}
