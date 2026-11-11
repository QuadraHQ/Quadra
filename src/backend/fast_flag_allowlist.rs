use super::paths;
use super::req_agent::api_agent;
use crate::utils::{create_parent_directory, decode_base64};
use anyhow::{Context, anyhow};
use serde_json::{Value, json};
use std::collections::HashSet;
use std::fs;
use std::sync::RwLock;

const SETTINGS_URL: &str =
    "https://clientsettingscdn.roblox.com/v2/settings/application/PCDesktopClient";
const ALLOWLIST_SETTING: &str = "DFStringAllowedPublicFlags";

static ALLOWED: RwLock<Option<HashSet<String>>> = RwLock::new(None);

pub fn initialize() {
    if let Some(cached) = read_cache() {
        remember(cached);
    }
    std::thread::spawn(|| match fetch() {
        Ok(allowed) => {
            write_cache(&allowed);
            remember(allowed);
        }
        Err(error) => eprintln!("Using the last known FastFlag allowlist: {error:#}"),
    });
}

pub fn is_ignored(name: &str) -> bool {
    ALLOWED
        .read()
        .ok()
        .and_then(|allowed| allowed.as_ref().map(|allowed| !allowed.contains(name)))
        .unwrap_or(false)
}

fn remember(allowed: HashSet<String>) {
    if let Ok(mut current) = ALLOWED.write() {
        *current = Some(allowed);
    }
}

fn fetch() -> anyhow::Result<HashSet<String>> {
    let settings: Value = api_agent()
        .get(SETTINGS_URL)
        .call()
        .and_then(|mut response| response.body_mut().read_json())
        .context("Couldn't download Roblox's FastFlag allowlist")?;

    let encoded = settings["applicationSettings"][ALLOWLIST_SETTING]
        .as_str()
        .ok_or_else(|| anyhow!("Roblox's settings have no FastFlag allowlist"))?;
    let decoded =
        decode_base64(encoded).ok_or_else(|| anyhow!("The FastFlag allowlist isn't base64"))?;
    let allowlist: Value = serde_json::from_slice(&decoded)?;

    let allowed: HashSet<String> = allowlist["Allowed"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|name| name.as_str().map(str::to_owned))
        .collect();
    if allowed.is_empty() {
        return Err(anyhow!("Roblox's FastFlag allowlist is empty"));
    }
    Ok(allowed)
}

fn read_cache() -> Option<HashSet<String>> {
    let contents = fs::read(paths::fast_flag_allowlist_file()).ok()?;
    let cached: Value = serde_json::from_slice(&contents).ok()?;
    let allowed: HashSet<String> = cached["allowed"]
        .as_array()?
        .iter()
        .filter_map(|name| name.as_str().map(str::to_owned))
        .collect();
    Some(allowed).filter(|allowed| !allowed.is_empty())
}

fn write_cache(allowed: &HashSet<String>) {
    let path = paths::fast_flag_allowlist_file();
    let mut names: Vec<&String> = allowed.iter().collect();
    names.sort();
    let written = create_parent_directory(&path)
        .and_then(|()| fs::write(&path, json!({ "allowed": names }).to_string()));
    if let Err(error) = written {
        eprintln!("Couldn't cache the FastFlag allowlist: {error}");
    }
}
