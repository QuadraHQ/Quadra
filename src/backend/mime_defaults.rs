use super::paths;
use anyhow::Context;
use gtk::glib;
use std::fs;
use std::io::ErrorKind;

const DEFAULT_APPLICATIONS: &str = "Default Applications";
const ADDED_ASSOCIATIONS: &str = "Added Associations";

pub fn is_default_for_all(desktop_file: &str, mime_types: &[&str]) -> bool {
    let Ok(defaults) = load() else {
        return false;
    };
    mime_types.iter().all(|mime_type| {
        defaults
            .string(DEFAULT_APPLICATIONS, mime_type)
            .is_ok_and(|default| default == desktop_file)
    })
}

pub fn make_default(desktop_file: &str, mime_types: &[&str]) -> anyhow::Result<()> {
    let defaults = load()?;
    for mime_type in mime_types {
        defaults.set_string(DEFAULT_APPLICATIONS, mime_type, desktop_file);

        let mut associations = associations_of(&defaults, mime_type);
        associations.retain(|association| association != desktop_file);
        associations.insert(0, desktop_file.to_owned());
        set_associations(&defaults, mime_type, &associations);
    }
    save(&defaults)
}

pub fn stop_being_default(desktop_file: &str, mime_types: &[&str]) -> anyhow::Result<()> {
    let defaults = load()?;
    for mime_type in mime_types {
        let is_default = defaults
            .string(DEFAULT_APPLICATIONS, mime_type)
            .is_ok_and(|default| default == desktop_file);
        if is_default {
            let _ = defaults.remove_key(DEFAULT_APPLICATIONS, mime_type);
        }

        let mut associations = associations_of(&defaults, mime_type);
        associations.retain(|association| association != desktop_file);
        if associations.is_empty() {
            let _ = defaults.remove_key(ADDED_ASSOCIATIONS, mime_type);
        } else {
            set_associations(&defaults, mime_type, &associations);
        }
    }
    save(&defaults)
}

fn associations_of(defaults: &glib::KeyFile, mime_type: &str) -> Vec<String> {
    defaults
        .string_list(ADDED_ASSOCIATIONS, mime_type)
        .map(|associations| associations.iter().map(|a| a.to_string()).collect())
        .unwrap_or_default()
}

fn set_associations(defaults: &glib::KeyFile, mime_type: &str, associations: &[String]) {
    let list: String = associations
        .iter()
        .map(|association| format!("{association};"))
        .collect();
    defaults.set_string(ADDED_ASSOCIATIONS, mime_type, &list);
}

fn load() -> anyhow::Result<glib::KeyFile> {
    let path = paths::mime_defaults_file();
    let defaults = glib::KeyFile::new();
    let contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(defaults),
        Err(error) => {
            return Err(error).with_context(|| format!("Couldn't read {}", path.display()));
        }
    };
    defaults
        .load_from_data(
            &contents,
            glib::KeyFileFlags::KEEP_COMMENTS | glib::KeyFileFlags::KEEP_TRANSLATIONS,
        )
        .with_context(|| format!("Couldn't understand {}", path.display()))?;
    Ok(defaults)
}

fn save(defaults: &glib::KeyFile) -> anyhow::Result<()> {
    let path = paths::mime_defaults_file();
    if let Some(directory) = path.parent() {
        fs::create_dir_all(directory)?;
    }
    fs::write(&path, defaults.to_data().as_str())
        .with_context(|| format!("Couldn't save {}", path.display()))
}
