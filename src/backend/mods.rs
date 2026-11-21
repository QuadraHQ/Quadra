mod content_files;
mod crash_guard;
mod cursor_presets;
mod custom_font;
mod folder_overlay;
mod settings;
mod sound_presets;

pub use settings::ModSettings;

use super::{paths, roblox};
use crate::state::Client;
use anyhow::Context;
use content_files::ContentFiles;
use std::fs;
use std::io;
use std::path::PathBuf;

const CONTENT_INSIDE_APP: &str = "Contents/Resources/content";

pub fn prepare(settings: &ModSettings) -> anyhow::Result<()> {
    let Some(release) = roblox::installed_release(Client::Player) else {
        return Ok(());
    };
    let content = roblox::app_path(Client::Player).join(CONTENT_INSIDE_APP);
    if !content.is_dir() {
        return Ok(());
    }

    forget_backups_of_other_versions(&release.version)?;
    let backups = paths::mods_backup_directory().join(&release.version);
    let mut files =
        ContentFiles::open_clean(content, backups).context("Couldn't undo the previous mods")?;

    report(
        "death sound",
        sound_presets::apply_death_sound(&mut files, settings),
    );
    report(
        "classic movement sounds",
        sound_presets::apply_old_character_sounds(&mut files, settings),
    );
    report("cursors", cursor_presets::apply(&mut files, settings));
    report(
        "font",
        custom_font::apply(&mut files, &settings.custom_font),
    );
    if settings.custom_mods {
        report(
            "modifications folder",
            folder_overlay::apply(&mut files, &paths::mods_directory()),
        );
    }
    files.finish().context("Couldn't record the applied mods")?;

    crash_guard::begin_session(&paths::mods_directory());
    Ok(())
}

pub fn finish_session(crashed: bool) -> Vec<String> {
    crash_guard::end_session(&paths::mods_directory(), crashed)
}

pub fn folder() -> io::Result<PathBuf> {
    let directory = paths::mods_directory();
    folder_overlay::create_folder(&directory)?;
    Ok(directory)
}

pub fn disabled_units() -> Vec<String> {
    crash_guard::disabled_units(&paths::mods_directory())
}

pub fn enable_unit(unit: &str) -> io::Result<()> {
    crash_guard::enable(&paths::mods_directory(), unit)
}

fn forget_backups_of_other_versions(current_version: &str) -> anyhow::Result<()> {
    let backups = paths::mods_backup_directory();
    let Ok(entries) = fs::read_dir(&backups) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        if entry.file_name().to_str() != Some(current_version) {
            fs::remove_dir_all(entry.path())?;
        }
    }
    Ok(())
}

fn report(step: &str, result: io::Result<()>) {
    if let Err(error) = result {
        eprintln!("Couldn't apply the mods for the {step}: {error}");
    }
}
