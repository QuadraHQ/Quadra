use super::{flatpak_override, paths};
use crate::utils::is_running_in_flatpak;

const BUTTER_RUNNER_PERMISSIONS: [&str; 2] = [
    "xdg-data/butter/runners:ro",
    "~/.var/app/io.github.microlift.butter/data/butter/runners:ro",
];

pub fn missing() -> Option<String> {
    let can_see_butter_runners = paths::butter_runner_directories()
        .iter()
        .any(|directory| directory.is_dir());
    if !is_running_in_flatpak() || can_see_butter_runners {
        return None;
    }
    Some(flatpak_override::command(
        &flatpak_override::quadra_id(),
        BUTTER_RUNNER_PERMISSIONS,
    ))
}
