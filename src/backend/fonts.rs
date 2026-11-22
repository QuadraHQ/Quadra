use crate::state::{AppState, ImportedFont, ImportedFonts};
use crate::utils::config_directory;
use anyhow::{Context, anyhow};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

const FONT_EXTENSIONS: [&str; 2] = ["ttf", "otf"];
const FAMILY_NAME: u16 = 1;
const TYPOGRAPHIC_FAMILY_NAME: u16 = 16;

pub fn folder() -> PathBuf {
    config_directory().join("quadra").join("fonts")
}

pub fn load(state: &AppState) {
    let fonts = font_files()
        .into_iter()
        .filter_map(|file| match read_font(&file) {
            Ok(font) => Some(font),
            Err(error) => {
                eprintln!("Couldn't use the font {}: {error:#}", file.display());
                None
            }
        })
        .collect();
    state.set_imported_fonts(ImportedFonts(Rc::new(fonts)));
}

pub fn import(state: &AppState, file: &Path) -> anyhow::Result<()> {
    let font = read_font(file)?;
    let file_name = file.file_name().context("The font file has no name")?;
    fs::create_dir_all(folder()).context("Couldn't make Quadra's fonts folder")?;
    fs::copy(file, folder().join(file_name)).context("Couldn't copy the font")?;
    load(state);
    state.set_ui_font(font.family);
    Ok(())
}

pub fn remove(state: &AppState, family: &str) -> anyhow::Result<()> {
    let fonts = state.imported_fonts();
    let font = fonts
        .0
        .iter()
        .find(|font| font.family == family)
        .with_context(|| format!("Couldn't find the file for the font {family}"))?;
    fs::remove_file(&font.file)
        .with_context(|| format!("Couldn't remove {}", font.file.display()))?;
    if state.ui_font() == family {
        state.set_ui_font("");
    }
    load(state);
    Ok(())
}

fn font_files() -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(folder()) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| is_font(path))
        .collect();
    files.sort();
    files
}

fn is_font(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            FONT_EXTENSIONS
                .iter()
                .any(|known| extension.eq_ignore_ascii_case(known))
        })
}

fn read_font(file: &Path) -> anyhow::Result<ImportedFont> {
    let bytes = fs::read(file).with_context(|| format!("Couldn't read {}", file.display()))?;
    let face = ttf_parser::Face::parse(&bytes, 0).context("It isn't a font Quadra can read")?;
    let weight = face.weight().to_number();
    let family = family_of(&face).ok_or_else(|| anyhow!("The font has no family name"))?;
    Ok(ImportedFont {
        family,
        weight,
        file: file.to_owned(),
        bytes: Arc::from(bytes),
    })
}

fn family_of(face: &ttf_parser::Face<'_>) -> Option<String> {
    let names = face.names();
    let named = |wanted: u16| {
        names
            .into_iter()
            .filter(|name| name.name_id == wanted && name.is_unicode())
            .find_map(|name| name.to_string())
    };
    named(TYPOGRAPHIC_FAMILY_NAME)
        .or_else(|| named(FAMILY_NAME))
        .filter(|family| !family.trim().is_empty())
}
