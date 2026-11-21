use crate::components::imported_theme::{ImportedTheme, parse_theme};
use crate::state::{AppState, ImportedThemes};
use crate::utils::config_directory;
use anyhow::{Context, anyhow};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

const THEME_EXTENSION: &str = "css";

pub fn folder() -> PathBuf {
    config_directory().join("quadra").join("themes")
}

pub fn load(state: &AppState) {
    let themes = theme_files()
        .into_iter()
        .filter_map(|file| match read_theme(&file) {
            Ok(theme) => {
                for problem in &theme.problems {
                    eprintln!("Theme {} skipped {problem}", file.display());
                }
                Some(theme)
            }
            Err(error) => {
                eprintln!("Couldn't use the theme {}: {error:#}", file.display());
                None
            }
        })
        .collect();
    state.set_imported_themes(ImportedThemes(Rc::new(themes)));
}

pub fn import(state: &AppState, file: &Path) -> anyhow::Result<()> {
    let theme = read_theme(file)?;
    let file_name = file.file_name().context("The theme file has no name")?;
    fs::create_dir_all(folder()).context("Couldn't make Quadra's themes folder")?;
    fs::copy(file, folder().join(file_name)).context("Couldn't copy the theme")?;
    load(state);
    state.set_imported_theme(theme.name);
    Ok(())
}

pub fn remove(state: &AppState, name: &str) -> anyhow::Result<()> {
    let file = theme_files()
        .into_iter()
        .find(|file| read_theme(file).is_ok_and(|theme| theme.name == name))
        .with_context(|| format!("Couldn't find the file for the theme {name}"))?;
    fs::remove_file(&file).with_context(|| format!("Couldn't remove {}", file.display()))?;
    if state.imported_theme() == name {
        state.set_imported_theme("");
    }
    load(state);
    Ok(())
}

fn theme_files() -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(folder()) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == THEME_EXTENSION)
        })
        .collect();
    files.sort();
    files
}

fn read_theme(file: &Path) -> anyhow::Result<ImportedTheme> {
    let css =
        fs::read_to_string(file).with_context(|| format!("Couldn't read {}", file.display()))?;
    let file_name = file
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    parse_theme(&file_name, &css).map_err(|problem| anyhow!(problem))
}
