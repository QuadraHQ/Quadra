use crate::utils::is_running_in_flatpak;
use gtk::glib;
use std::path::PathBuf;

pub fn quadra_data_directory() -> PathBuf {
    glib::user_data_dir().join("quadra")
}

pub fn quadra_rosemary_directory(version: &str) -> PathBuf {
    quadra_data_directory().join("rosemary").join(version)
}

pub fn quadra_prefix_directory() -> PathBuf {
    quadra_data_directory().join("prefix")
}

pub fn roblox_directory() -> PathBuf {
    quadra_data_directory().join("roblox")
}

pub fn studio_directory() -> PathBuf {
    quadra_data_directory().join("studio")
}

pub fn applications_directory() -> PathBuf {
    host_data_directory().join("applications")
}

pub fn mime_directory() -> PathBuf {
    host_data_directory().join("mime")
}

pub fn mime_defaults_file() -> PathBuf {
    if is_running_in_flatpak() {
        glib::home_dir().join(".config/mimeapps.list")
    } else {
        glib::user_config_dir().join("mimeapps.list")
    }
}

fn host_data_directory() -> PathBuf {
    if is_running_in_flatpak() {
        glib::home_dir().join(".local/share")
    } else {
        glib::user_data_dir()
    }
}

pub fn desktop_entry_icon_file() -> PathBuf {
    quadra_data_directory()
        .join("icons")
        .join("org.quadrahq.Quadra.png")
}

pub fn logs_directory() -> PathBuf {
    quadra_data_directory().join("logs")
}

pub fn downloads_directory() -> PathBuf {
    glib::user_cache_dir().join("quadra").join("downloads")
}

pub fn butter_runner_directories() -> [PathBuf; 2] {
    [
        glib::user_data_dir().join("butter").join("runners"),
        glib::home_dir().join(".var/app/io.github.microlift.butter/data/butter/runners"),
    ]
}

pub fn server_history_file() -> PathBuf {
    glib::user_config_dir()
        .join("quadra")
        .join("server-history.json")
}

pub fn regions_cache_file() -> PathBuf {
    glib::user_cache_dir()
        .join("quadra")
        .join("rovalra-regions.json")
}

pub fn vr_directory() -> PathBuf {
    glib::user_cache_dir().join("quadra").join("vr")
}

pub fn mods_directory() -> PathBuf {
    quadra_data_directory().join("modifications")
}

pub fn mods_backup_directory() -> PathBuf {
    quadra_data_directory().join("mods-backup")
}

pub fn mod_guard_file() -> PathBuf {
    glib::user_config_dir()
        .join("quadra")
        .join("mod-guard.json")
}

pub fn fast_flag_allowlist_file() -> PathBuf {
    glib::user_cache_dir()
        .join("quadra")
        .join("fast-flag-allowlist.json")
}
