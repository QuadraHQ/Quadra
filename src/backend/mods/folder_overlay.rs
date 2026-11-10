use super::content_files::ContentFiles;
use super::crash_guard;
use crate::utils::files_under;
use std::io;
use std::path::Path;

const IGNORED_NAMES: [&str; 2] = ["README.txt", ".DS_Store"];
const DELETE_MARKER: &str = "_Delete";
const README: &str = "Place custom files here to override Roblox content.\n\
For example: textures/Cursors/KeyboardMouse/ArrowCursor.png\n\
or sounds/ouch.ogg\n\
Name a file like ouch_Delete.ogg to remove the original sounds/ouch.ogg.\n";

pub fn create_folder(mods_directory: &Path) -> io::Result<()> {
    std::fs::create_dir_all(mods_directory)?;
    let readme = mods_directory.join("README.txt");
    if !readme.exists() {
        std::fs::write(readme, README)?;
    }
    Ok(())
}

pub fn apply(files: &mut ContentFiles, mods_directory: &Path) -> io::Result<()> {
    for unit in crash_guard::active_units(mods_directory) {
        for source in files_under(&mods_directory.join(unit))? {
            let is_ignored = source
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| IGNORED_NAMES.contains(&name));
            let Ok(relative) = source.strip_prefix(mods_directory) else {
                continue;
            };
            if is_ignored {
                continue;
            }
            match deletion_target(relative) {
                Some(target) => files.remove(&target)?,
                None => files.place_file(relative, &source)?,
            }
        }
    }
    Ok(())
}

fn deletion_target(relative: &Path) -> Option<std::path::PathBuf> {
    let stem = relative.file_stem()?.to_str()?;
    let original_stem = stem.strip_suffix(DELETE_MARKER)?;
    let name = match relative.extension() {
        Some(extension) => format!("{original_stem}.{}", extension.to_string_lossy()),
        None => original_stem.to_owned(),
    };
    Some(relative.with_file_name(name))
}
