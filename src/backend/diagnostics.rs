mod archive;
mod redaction;
mod system_report;

use crate::state::{AppState, settings_path};
use anyhow::Context;
use archive::Archive;
use redaction::redact;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const LOG_TAIL_BYTES: u64 = 2 * 1024 * 1024;
const SECRET_SETTINGS: [&str; 1] = ["environment_variables"];

pub fn hardware_report() -> String {
    system_report::hardware_section()
}

pub fn quadra_summary(state: &AppState) -> String {
    system_report::quadra_section(state)
}

pub fn collect(quadra_summary: &str, directory: &Path) -> anyhow::Result<PathBuf> {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    let path = directory.join(format!("quadra-diagnostics-{seconds}.tar.gz"));

    let report = format!(
        "{}\n{}\nEnvironment\n{}",
        quadra_summary,
        system_report::hardware_section(),
        system_report::environment_section()
    );

    let mut archive = Archive::create(&path)?;
    archive.add("report.txt", &report)?;
    archive.add("settings.toml", &settings_without_secrets())?;
    archive.add("roblox.log", &redact(&log_tail(&system_report::log_file())))?;
    archive.finish()?;
    Ok(path)
}

fn settings_without_secrets() -> String {
    let settings = fs::read_to_string(settings_path()).unwrap_or_default();
    let Ok(mut settings) = toml::from_str::<toml::Table>(&settings) else {
        return String::new();
    };
    for secret in SECRET_SETTINGS {
        settings.remove(secret);
    }
    toml::to_string_pretty(&settings).unwrap_or_default()
}

fn log_tail(path: &Path) -> String {
    let read = || -> anyhow::Result<String> {
        let mut file = File::open(path)?;
        let length = file.metadata()?.len();
        file.seek(SeekFrom::Start(length.saturating_sub(LOG_TAIL_BYTES)))?;
        let mut tail = Vec::new();
        file.read_to_end(&mut tail)?;
        Ok(String::from_utf8_lossy(&tail).into_owned())
    };
    read()
        .with_context(|| format!("Couldn't read {}", path.display()))
        .unwrap_or_else(|error| format!("<{error:#}>"))
}
