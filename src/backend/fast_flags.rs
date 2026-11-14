use super::{paths, roblox};
use crate::state::{AppState, Client, FastFlagValue, FastFlags};
use anyhow::Context;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const CLIENT_SETTINGS_INSIDE_APP: &str = "Contents/MacOS/ClientSettings/ClientAppSettings.json";

pub fn initialize(state: &AppState) {
    state.follow_properties(&["fast-flags"], |state| {
        if let Err(error) = remember(&state.fast_flags()) {
            eprintln!("{error:#}");
        }
    });
}

pub fn write_client_settings(flags: &FastFlags) -> anyhow::Result<()> {
    write(&client_settings_in_app(), flags)
}

pub fn reapply_remembered() -> anyhow::Result<()> {
    let remembered = paths::fast_flags_file();
    if !remembered.is_file() {
        return Ok(());
    }
    let destination = client_settings_in_app();
    create_parent_of(&destination)?;
    fs::copy(&remembered, &destination).with_context(|| {
        format!(
            "Couldn't put the FastFlags back into {}",
            destination.display()
        )
    })?;
    Ok(())
}

fn remember(flags: &FastFlags) -> anyhow::Result<()> {
    write(&paths::fast_flags_file(), flags)?;
    if roblox::installed_release(Client::Player).is_some() {
        write_client_settings(flags)?;
    }
    Ok(())
}

fn client_settings_in_app() -> PathBuf {
    roblox::app_path(Client::Player).join(CLIENT_SETTINGS_INSIDE_APP)
}

fn write(path: &Path, flags: &FastFlags) -> anyhow::Result<()> {
    let client_settings: BTreeMap<&str, String> = flags
        .0
        .iter()
        .map(|(name, value)| (name.as_str(), client_setting_value(value)))
        .collect();

    create_parent_of(path)?;
    fs::write(path, serde_json::to_string_pretty(&client_settings)?)
        .with_context(|| format!("Couldn't write the FastFlags to {}", path.display()))
}

fn create_parent_of(path: &Path) -> anyhow::Result<()> {
    if let Some(directory) = path.parent() {
        fs::create_dir_all(directory)
            .with_context(|| format!("Couldn't create {}", directory.display()))?;
    }
    Ok(())
}

fn client_setting_value(value: &FastFlagValue) -> String {
    match value {
        FastFlagValue::Toggle(true) => "True".to_owned(),
        FastFlagValue::Toggle(false) => "False".to_owned(),
        FastFlagValue::Number(number) => number.to_string(),
        FastFlagValue::Text(text) => text.clone(),
    }
}
