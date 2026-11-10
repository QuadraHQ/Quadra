use super::content_files::ContentFiles;
use super::settings::ModSettings;
use crate::state::CursorStyle;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::Path;

const CURSOR_DIRECTORY: &str = "textures/Cursors/KeyboardMouse";
const ARROW_CURSORS: [&str; 2] = ["ArrowCursor.png", "ArrowFarCursor.png"];

struct Cursor {
    name: &'static str,
    image: &'static [u8],
}

macro_rules! cursor {
    ($style:literal, $name:literal) => {
        Cursor {
            name: $name,
            image: include_bytes!(concat!("../../../data/mods/cursors/", $style, "/", $name)),
        }
    };
}

pub fn apply(files: &mut ContentFiles, settings: &ModSettings) -> io::Result<()> {
    match settings.cursor_style {
        CursorStyle::Stock => Ok(()),
        CursorStyle::Custom => apply_custom(files, &settings.custom_cursor),
        style => preset_of(style).iter().try_for_each(|cursor| {
            files.place_bytes(&Path::new(CURSOR_DIRECTORY).join(cursor.name), cursor.image)
        }),
    }
}

fn preset_of(style: CursorStyle) -> Vec<Cursor> {
    match style {
        CursorStyle::Classic2006 => vec![
            cursor!("2006", "ArrowCursor.png"),
            cursor!("2006", "ArrowFarCursor.png"),
        ],
        CursorStyle::Classic2013 => vec![
            cursor!("2013", "ArrowCursor.png"),
            cursor!("2013", "ArrowFarCursor.png"),
        ],
        CursorStyle::PurpleCross => vec![
            cursor!("purple_cross", "ArrowCursor.png"),
            cursor!("purple_cross", "ArrowFarCursor.png"),
            cursor!("purple_cross", "IBeamCursor.png"),
        ],
        CursorStyle::Dot => vec![
            cursor!("dot", "ArrowCursor.png"),
            cursor!("dot", "ArrowFarCursor.png"),
            cursor!("dot", "IBeamCursor.png"),
        ],
        CursorStyle::Stock | CursorStyle::Custom => Vec::new(),
    }
}

fn apply_custom(files: &mut ContentFiles, chosen: &Path) -> io::Result<()> {
    let cursor_directory = Path::new(CURSOR_DIRECTORY);
    if is_png(chosen) {
        return ARROW_CURSORS
            .iter()
            .try_for_each(|name| files.place_file(&cursor_directory.join(name), chosen));
    }
    if !chosen.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(chosen)? {
        let image = entry?.path();
        if let (true, Some(name)) = (is_png(&image), image.file_name()) {
            files.place_file(&cursor_directory.join(name), &image)?;
        }
    }
    Ok(())
}

fn is_png(path: &Path) -> bool {
    path.is_file()
        && path
            .extension()
            .and_then(OsStr::to_str)
            .is_some_and(|extension| extension.eq_ignore_ascii_case("png"))
}
