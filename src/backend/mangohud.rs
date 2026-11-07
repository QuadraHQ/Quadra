use crate::utils::find_in_path;
use std::path::Path;

const FLATPAK_EXTENSION_DIRECTORY: &str = "/usr/lib/extensions/vulkan/MangoHud";

pub fn is_installed() -> bool {
    find_in_path("mangohud").is_some() || Path::new(FLATPAK_EXTENSION_DIRECTORY).is_dir()
}
