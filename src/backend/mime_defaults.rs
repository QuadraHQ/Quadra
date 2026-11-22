use super::key_file::KeyFile;
use super::paths;
use anyhow::Context;
use std::fs;
use std::io::ErrorKind;

const DEFAULT_APPLICATIONS: &str = "Default Applications";
const ADDED_ASSOCIATIONS: &str = "Added Associations";

pub fn is_default_for_all(desktop_file: &str, mime_types: &[&str]) -> bool {
    let Ok(defaults) = load() else {
        return false;
    };
    mime_types
        .iter()
        .all(|mime_type| defaults.value(DEFAULT_APPLICATIONS, mime_type) == Some(desktop_file))
}

pub fn make_default(desktop_file: &str, mime_types: &[&str]) -> anyhow::Result<()> {
    let mut defaults = load()?;
    for mime_type in mime_types {
        defaults.set_value(DEFAULT_APPLICATIONS, mime_type, desktop_file);

        let mut associations = associations_of(&defaults, mime_type);
        associations.retain(|association| association != desktop_file);
        associations.insert(0, desktop_file.to_owned());
        set_associations(&mut defaults, mime_type, &associations);
    }
    save(&defaults)
}

pub fn stop_being_default(desktop_file: &str, mime_types: &[&str]) -> anyhow::Result<()> {
    let mut defaults = load()?;
    for mime_type in mime_types {
        if defaults.value(DEFAULT_APPLICATIONS, mime_type) == Some(desktop_file) {
            defaults.remove(DEFAULT_APPLICATIONS, mime_type);
        }

        let mut associations = associations_of(&defaults, mime_type);
        associations.retain(|association| association != desktop_file);
        if associations.is_empty() {
            defaults.remove(ADDED_ASSOCIATIONS, mime_type);
        } else {
            set_associations(&mut defaults, mime_type, &associations);
        }
    }
    save(&defaults)
}

fn associations_of(defaults: &KeyFile, mime_type: &str) -> Vec<String> {
    defaults
        .value(ADDED_ASSOCIATIONS, mime_type)
        .unwrap_or_default()
        .split(';')
        .map(str::trim)
        .filter(|association| !association.is_empty())
        .map(str::to_owned)
        .collect()
}

fn set_associations(defaults: &mut KeyFile, mime_type: &str, associations: &[String]) {
    let list: String = associations
        .iter()
        .map(|association| format!("{association};"))
        .collect();
    defaults.set_value(ADDED_ASSOCIATIONS, mime_type, &list);
}

fn load() -> anyhow::Result<KeyFile> {
    let path = paths::mime_defaults_file();
    match fs::read_to_string(&path) {
        Ok(contents) => Ok(KeyFile::parse(&contents)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(KeyFile::default()),
        Err(error) => Err(error).with_context(|| format!("Couldn't read {}", path.display())),
    }
}

fn save(defaults: &KeyFile) -> anyhow::Result<()> {
    let path = paths::mime_defaults_file();
    if let Some(directory) = path.parent() {
        fs::create_dir_all(directory)?;
    }
    fs::write(&path, defaults.to_string())
        .with_context(|| format!("Couldn't save {}", path.display()))
}
