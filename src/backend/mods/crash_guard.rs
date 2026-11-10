use crate::backend::paths;
use crate::utils::create_parent_directory;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const SETTLE_AFTER: Duration = Duration::from_secs(300);
const DISABLED_SUFFIX: &str = ".disabled";
const IGNORED_NAMES: [&str; 2] = ["README.txt", ".DS_Store"];

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct Guard {
    session: Option<Session>,
    last_good: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize)]
struct Session {
    units: Vec<String>,
    started: u64,
}

pub fn active_units(mods_directory: &Path) -> Vec<String> {
    names_in(mods_directory)
        .into_iter()
        .filter(|name| !IGNORED_NAMES.contains(&name.as_str()) && !name.ends_with(DISABLED_SUFFIX))
        .collect()
}

pub fn disabled_units(mods_directory: &Path) -> Vec<String> {
    names_in(mods_directory)
        .into_iter()
        .filter_map(|name| name.strip_suffix(DISABLED_SUFFIX).map(str::to_owned))
        .collect()
}

pub fn begin_session(mods_directory: &Path) {
    let mut guard = load();
    guard.session = Some(Session {
        units: active_units(mods_directory),
        started: now(),
    });
    save(&guard);
}

pub fn end_session(mods_directory: &Path, crashed: bool) -> Vec<String> {
    let mut guard = load();
    let Some(session) = guard.session.take() else {
        return Vec::new();
    };

    let ran_for = Duration::from_secs(now().saturating_sub(session.started));
    let crashed_early = crashed && ran_for < SETTLE_AFTER;
    let mut set_aside = Vec::new();
    match (&guard.last_good, crashed_early) {
        (Some(last_good), true) => {
            for unit in session
                .units
                .iter()
                .filter(|unit| !last_good.contains(unit))
            {
                if disable(mods_directory, unit).is_ok() {
                    set_aside.push(unit.clone());
                }
            }
        }
        (None, true) => {}
        (_, false) => guard.last_good = Some(session.units),
    }
    save(&guard);
    set_aside
}

pub fn enable(mods_directory: &Path, unit: &str) -> io::Result<()> {
    let disabled = mods_directory.join(format!("{unit}{DISABLED_SUFFIX}"));
    let enabled = mods_directory.join(unit);
    if enabled.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{unit} already exists in the modifications folder."),
        ));
    }
    fs::rename(disabled, enabled)
}

fn disable(mods_directory: &Path, unit: &str) -> io::Result<()> {
    let enabled = mods_directory.join(unit);
    let disabled = mods_directory.join(format!("{unit}{DISABLED_SUFFIX}"));
    if disabled.exists() {
        return Err(io::Error::from(io::ErrorKind::AlreadyExists));
    }
    fs::rename(enabled, disabled)
}

fn names_in(directory: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(directory)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();
    names.sort();
    names
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

fn load() -> Guard {
    fs::read(paths::mod_guard_file())
        .ok()
        .and_then(|contents| serde_json::from_slice(&contents).ok())
        .unwrap_or_default()
}

fn save(guard: &Guard) {
    let path = paths::mod_guard_file();
    let unfinished = path.with_extension("json.partial");
    let written = create_parent_directory(&path)
        .and_then(|()| fs::write(&unfinished, serde_json::to_vec(guard)?))
        .and_then(|()| fs::rename(&unfinished, &path));
    if let Err(error) = written {
        eprintln!("Couldn't save the mod crash guard state: {error}");
    }
}
