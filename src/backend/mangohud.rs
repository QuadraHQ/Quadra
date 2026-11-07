use crate::utils::{find_in_path, is_executable_file};
use std::path::{Path, PathBuf};

const FLATPAK_EXTENSION_EXECUTABLE: &str = "/usr/lib/extensions/vulkan/MangoHud/bin/mangohud";

pub fn executable() -> Option<PathBuf> {
    find_in_path("mangohud").or_else(|| {
        let flatpak_extension_executable = Path::new(FLATPAK_EXTENSION_EXECUTABLE);
        is_executable_file(flatpak_extension_executable)
            .then(|| flatpak_extension_executable.to_owned())
    })
}
