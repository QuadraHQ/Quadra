use super::content_files::ContentFiles;
use super::settings::ModSettings;
use crate::state::FontStyle;
use serde_json::Value;
use std::ffi::OsStr;
use std::io;
use std::path::Path;

const FONTS_DIRECTORY: &str = "fonts";
const FAMILIES_DIRECTORY: &str = "fonts/families";

macro_rules! preset {
    ($folder:literal) => {
        include_bytes!(concat!("../../../data/mods/fonts/", $folder, "/Font.ttf"))
    };
}

pub fn apply(files: &mut ContentFiles, settings: &ModSettings) -> io::Result<()> {
    let preset: &[u8] = match settings.font_style {
        FontStyle::Stock => return Ok(()),
        FontStyle::Custom => return apply_file(files, &settings.custom_font),
        FontStyle::ComicNeue => preset!("comic_neue"),
        FontStyle::PressStart => preset!("press_start"),
        FontStyle::Bangers => preset!("bangers"),
        FontStyle::Vt323 => preset!("vt323"),
        FontStyle::PatrickHand => preset!("patrick_hand"),
        FontStyle::AtkinsonHyperlegible => preset!("atkinson_hyperlegible"),
    };
    let name = "CustomFont.ttf";
    files.place_bytes(&Path::new(FONTS_DIRECTORY).join(name), preset)?;
    use_font_everywhere(files, name)
}

fn apply_file(files: &mut ContentFiles, font: &Path) -> io::Result<()> {
    let extension = font
        .extension()
        .and_then(OsStr::to_str)
        .map(str::to_ascii_lowercase);
    let Some(extension) = extension.filter(|extension| extension == "ttf" || extension == "otf")
    else {
        return Ok(());
    };
    if !font.is_file() {
        return Ok(());
    }

    let name = format!("CustomFont.{extension}");
    files.place_file(&Path::new(FONTS_DIRECTORY).join(&name), font)?;
    use_font_everywhere(files, &name)
}

fn use_font_everywhere(files: &mut ContentFiles, name: &str) -> io::Result<()> {
    let asset_uri = format!("rbxasset://fonts/{name}");
    for family in files.list(Path::new(FAMILIES_DIRECTORY)) {
        if family
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            use_font_in_family(files, &family, &asset_uri)?;
        }
    }
    Ok(())
}

fn use_font_in_family(files: &mut ContentFiles, family: &Path, asset_uri: &str) -> io::Result<()> {
    let Ok(mut description) = serde_json::from_slice::<Value>(&files.read(family)?) else {
        return Ok(());
    };
    let Some(faces) = description.get_mut("faces").and_then(Value::as_array_mut) else {
        return Ok(());
    };
    for face in faces.iter_mut().filter_map(Value::as_object_mut) {
        face.insert("assetId".to_owned(), asset_uri.into());
    }
    let patched = serde_json::to_vec_pretty(&description).map_err(io::Error::other)?;
    files.place_bytes(family, &patched)
}
