use crate::utils::is_running_in_flatpak;
use std::ffi::OsString;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallKind {
    AppImage(PathBuf),
    Flatpak,
    Other,
}

impl InstallKind {
    pub fn detect() -> Self {
        Self::from_environment(std::env::var_os("APPIMAGE"), is_running_in_flatpak())
    }

    fn from_environment(appimage: Option<OsString>, in_flatpak: bool) -> Self {
        if in_flatpak {
            return Self::Flatpak;
        }
        match appimage.filter(|path| !path.is_empty()).map(PathBuf::from) {
            Some(path) if path.is_file() => Self::AppImage(path),
            _ => Self::Other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::InstallKind;

    #[test]
    fn tells_the_ways_quadra_is_installed_apart() {
        let existing = std::env::current_exe().unwrap();
        assert_eq!(
            InstallKind::from_environment(Some(existing.clone().into_os_string()), false),
            InstallKind::AppImage(existing.clone())
        );
        assert_eq!(
            InstallKind::from_environment(Some(existing.into_os_string()), true),
            InstallKind::Flatpak
        );
        assert_eq!(
            InstallKind::from_environment(None, false),
            InstallKind::Other
        );
        assert_eq!(
            InstallKind::from_environment(Some("/nowhere/Quadra.AppImage".into()), false),
            InstallKind::Other
        );
        assert_eq!(
            InstallKind::from_environment(Some("".into()), false),
            InstallKind::Other
        );
    }
}
