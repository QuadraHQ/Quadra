use super::background::Progress;
use super::paths;
use crate::state::QUADRA_PREFIX;
use anyhow::{Context, bail};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const STATE_FILE: &str = ".rosemary/state.yaml";
const DARWIN_BASE_ENTRY: &str = "base:";
const PINNED_RELEASE_ENTRY: &str = "pinned:";
const PINNED_RELEASE_IS_NEWER: &str = "`rosemary prefix update` installs it";
const LINES_OF_ERROR_OUTPUT_TO_SHOW: usize = 10;
const ROOT_INSIDE_PREFIX: &str = "root";
const HOST_ROOT_SEEN_FROM_MACOS: &str = "/Volumes/Host";

pub fn directory_for(prefix_name: &str) -> Option<PathBuf> {
    (prefix_name == QUADRA_PREFIX).then(paths::quadra_prefix_directory)
}

pub fn path_seen_from_macos(prefix_directory: Option<&Path>, host_path: &Path) -> PathBuf {
    let path_inside_prefix = prefix_directory.and_then(|prefix_directory| {
        host_path
            .strip_prefix(prefix_directory.join(ROOT_INSIDE_PREFIX))
            .ok()
    });

    match path_inside_prefix {
        Some(path_inside_prefix) => Path::new("/").join(path_inside_prefix),
        None => Path::new(HOST_ROOT_SEEN_FROM_MACOS)
            .join(host_path.strip_prefix("/").unwrap_or(host_path)),
    }
}

pub fn has_darwin_base(prefix_directory: &Path) -> bool {
    fs::read_to_string(prefix_directory.join(STATE_FILE)).is_ok_and(|state| {
        state
            .lines()
            .any(|line| line.starts_with(DARWIN_BASE_ENTRY))
    })
}

pub fn install_darwin_base(rosemary: &Path, prefix_directory: &Path) -> anyhow::Result<()> {
    run_prefix_command(rosemary, "create", prefix_directory)
}

pub fn repair(rosemary: &Path, prefix_directory: &Path) -> anyhow::Result<()> {
    run_prefix_command(rosemary, "repair", prefix_directory)
}

pub fn update_darwin_base(rosemary: &Path, prefix_directory: &Path) -> anyhow::Result<()> {
    run_prefix_command(rosemary, "update", prefix_directory)
}

pub fn update_darwin_base_if_outdated(
    rosemary: &Path,
    prefix_directory: &Path,
    progress: &Progress,
) -> anyhow::Result<()> {
    let Some(newer_release) = newer_darwin_base(rosemary, prefix_directory)? else {
        return Ok(());
    };
    progress.step(format!("Updating the Darwin base to {newer_release}"));
    update_darwin_base(rosemary, prefix_directory)
}

pub fn newer_darwin_base(
    rosemary: &Path,
    prefix_directory: &Path,
) -> anyhow::Result<Option<String>> {
    let output = Command::new(rosemary)
        .args(["prefix", "info"])
        .arg(prefix_directory)
        .output()
        .with_context(|| format!("Couldn't start {}", rosemary.display()))?;
    if !output.status.success() {
        bail!(
            "`rosemary prefix info` failed ({}):\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(newer_release_in(&String::from_utf8_lossy(&output.stdout)))
}

fn newer_release_in(prefix_info: &str) -> Option<String> {
    let pinned_release = prefix_info
        .lines()
        .find_map(|line| line.strip_prefix(PINNED_RELEASE_ENTRY))?;
    if !pinned_release.contains(PINNED_RELEASE_IS_NEWER) {
        return None;
    }
    let (release, _) = pinned_release.split_once(';')?;
    Some(release.trim().to_owned())
}

fn run_prefix_command(
    rosemary: &Path,
    action: &str,
    prefix_directory: &Path,
) -> anyhow::Result<()> {
    let output = Command::new(rosemary)
        .args(["prefix", action])
        .arg(prefix_directory)
        .env("ROSEMARY_PREFIX_SETUP", "download")
        .output()
        .with_context(|| format!("Couldn't start {}", rosemary.display()))?;

    if !output.status.success() {
        let error_output = String::from_utf8_lossy(&output.stderr);
        let last_lines: Vec<&str> = error_output
            .lines()
            .rev()
            .take(LINES_OF_ERROR_OUTPUT_TO_SHOW)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        bail!(
            "`rosemary prefix {action}` failed ({}):\n{}",
            output.status,
            last_lines.join("\n")
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_newer_pinned_release() {
        let info = "dirty:    no\npinned:   release 17; `rosemary prefix update` installs it\n";
        assert_eq!(newer_release_in(info).as_deref(), Some("release 17"));
    }

    #[test]
    fn finds_nothing_when_up_to_date() {
        let info = "dirty:    no\npinned:   release 15; up to date\n";
        assert_eq!(newer_release_in(info), None);
    }
}
