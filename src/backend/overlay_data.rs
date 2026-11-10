use super::paths;
use crate::state::OverlayLayout;
use crate::utils::write_file_atomically;
use std::fs;
use std::io::ErrorKind;

pub fn read_layout() -> OverlayLayout {
    let path = paths::overlay_layout_file();
    match fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_else(|error| {
            eprintln!(
                "Ignoring the invalid overlay layout in {}: {error}",
                path.display()
            );
            OverlayLayout::default()
        }),
        Err(error) if error.kind() == ErrorKind::NotFound => OverlayLayout::default(),
        Err(error) => {
            eprintln!(
                "Couldn't read the overlay layout from {}: {error}",
                path.display()
            );
            OverlayLayout::default()
        }
    }
}

pub fn write_layout(layout: &OverlayLayout) {
    let path = paths::overlay_layout_file();
    let written = serde_json::to_string_pretty(layout)
        .map_err(std::io::Error::other)
        .and_then(|contents| write_file_atomically(&path, &contents));
    if let Err(error) = written {
        eprintln!(
            "Couldn't save the overlay layout to {}: {error}",
            path.display()
        );
    }
}

pub fn read_notes() -> String {
    fs::read_to_string(paths::overlay_notes_file()).unwrap_or_default()
}

pub fn write_notes(notes: &str) -> bool {
    let path = paths::overlay_notes_file();
    match write_file_atomically(&path, notes) {
        Ok(()) => true,
        Err(error) => {
            eprintln!("Couldn't save the notes to {}: {error}", path.display());
            false
        }
    }
}
