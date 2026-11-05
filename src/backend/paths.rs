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
