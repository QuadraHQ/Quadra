use super::{paths, updater};
use crate::utils::{APP_ICON_PNG, is_running_in_flatpak};
use anyhow::Context;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Opens {
    Nothing,
    Link,
    File,
}

pub struct DesktopEntry {
    pub file_name: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub command_line_option: Option<&'static str>,
    pub opens: Opens,
    pub mime_types: &'static [&'static str],
    pub shown_in_app_menu: bool,
}

enum Launcher {
    Flatpak { app_id: String },
    Executable(PathBuf),
}

impl Launcher {
    fn current() -> anyhow::Result<Self> {
        if is_running_in_flatpak() {
            let app_id =
                std::env::var("FLATPAK_ID").context("Couldn't find Quadra's Flatpak ID")?;
            return Ok(Self::Flatpak { app_id });
        }
        match updater::running_appimage() {
            Some(appimage) => Ok(Self::Executable(appimage)),
            None => std::env::current_exe()
                .map(Self::Executable)
                .context("Couldn't find Quadra's executable"),
        }
    }
}

impl DesktopEntry {
    fn path(&self) -> PathBuf {
        paths::applications_directory().join(self.file_name)
    }

    pub fn exists(&self) -> bool {
        self.path().is_file()
    }

    pub fn add(&self) -> anyhow::Result<()> {
        self.write(&Launcher::current()?)
    }

    pub fn refresh(&self) -> anyhow::Result<()> {
        if !self.exists() {
            return Ok(());
        }
        let launcher = Launcher::current()?;
        let wanted = self.contents(&paths::desktop_entry_icon_file(), &launcher);
        let is_up_to_date = fs::read_to_string(self.path()).is_ok_and(|written| written == wanted);
        if is_up_to_date {
            return Ok(());
        }
        self.write(&launcher)
    }

    fn write(&self, launcher: &Launcher) -> anyhow::Result<()> {
        let icon = paths::desktop_entry_icon_file();
        if let Some(directory) = icon.parent() {
            fs::create_dir_all(directory)?;
        }
        fs::write(&icon, APP_ICON_PNG)
            .with_context(|| format!("Couldn't save the icon to {}", icon.display()))?;

        let path = self.path();
        if let Some(directory) = path.parent() {
            fs::create_dir_all(directory)?;
        }
        fs::write(&path, self.contents(&icon, launcher))
            .with_context(|| format!("Couldn't add {}", path.display()))
    }

    pub fn remove(&self) -> anyhow::Result<()> {
        let path = self.path();
        match fs::remove_file(&path) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                Err(error).with_context(|| format!("Couldn't remove {}", path.display()))
            }
            _ => Ok(()),
        }
    }

    fn contents(&self, icon: &Path, launcher: &Launcher) -> String {
        let mut contents = format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Name={}\n\
             Comment={}\n\
             Exec={}\n\
             Icon={}\n\
             Terminal=false\n\
             Categories=Game;\n\
             StartupNotify=true\n",
            self.name,
            self.description,
            self.exec(launcher),
            icon.display()
        );
        if !self.mime_types.is_empty() {
            contents.push_str(&format!("MimeType={};\n", self.mime_types.join(";")));
        }
        if !self.shown_in_app_menu {
            contents.push_str("NoDisplay=true\n");
        }
        contents
    }

    fn exec(&self, launcher: &Launcher) -> String {
        let mut exec = match launcher {
            Launcher::Flatpak { app_id } if self.opens == Opens::File => {
                format!("flatpak run --file-forwarding {app_id}")
            }
            Launcher::Flatpak { app_id } => format!("flatpak run {app_id}"),
            Launcher::Executable(executable) => quoted_for_exec(&executable.display().to_string()),
        };

        if let Some(option) = self.command_line_option {
            exec.push(' ');
            exec.push_str(option);
        }

        match (self.opens, launcher) {
            (Opens::Nothing, _) => {}
            (Opens::Link, _) => exec.push_str(" %u"),
            (Opens::File, Launcher::Flatpak { .. }) => exec.push_str(" @@ %f @@"),
            (Opens::File, Launcher::Executable(_)) => exec.push_str(" %f"),
        }

        exec
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Shortcut {
    Quadra,
    Roblox,
    Studio,
}

impl Shortcut {
    pub const ALL: [Self; 3] = [Self::Quadra, Self::Roblox, Self::Studio];

    pub fn entry(self) -> DesktopEntry {
        let (file_name, name, description, command_line_option) = match self {
            Self::Quadra => (
                "org.quadrahq.Quadra.desktop",
                "Quadra",
                "Opens Quadra",
                None,
            ),
            Self::Roblox => (
                "org.quadrahq.Quadra.Roblox.desktop",
                "Roblox",
                "Starts Roblox without opening Quadra",
                Some("--roblox"),
            ),
            Self::Studio => (
                "org.quadrahq.Quadra.Studio.desktop",
                "Roblox Studio",
                "Starts Roblox Studio without opening Quadra",
                Some("--studio"),
            ),
        };

        DesktopEntry {
            file_name,
            name,
            description,
            command_line_option,
            opens: Opens::Nothing,
            mime_types: &[],
            shown_in_app_menu: true,
        }
    }
}

fn quoted_for_exec(argument: &str) -> String {
    let argument = argument.replace('%', "%%");
    let needs_quotes = argument
        .chars()
        .any(|character| character.is_whitespace() || "\"'\\$`<>|&;*?#()".contains(character));
    if !needs_quotes {
        return argument;
    }

    let escaped: String = argument
        .chars()
        .flat_map(|character| match character {
            '"' | '`' | '$' | '\\' => vec!['\\', character],
            _ => vec![character],
        })
        .collect();
    format!("\"{}\"", escaped.replace('\\', "\\\\"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(opens: Opens, command_line_option: Option<&'static str>) -> DesktopEntry {
        DesktopEntry {
            file_name: "test.desktop",
            name: "Test",
            description: "Test",
            command_line_option,
            opens,
            mime_types: &[],
            shown_in_app_menu: false,
        }
    }

    #[test]
    fn appimage_entries_start_the_appimage_itself() {
        let launcher = Launcher::Executable(PathBuf::from("/home/someone/Apps/Quadra.AppImage"));
        assert_eq!(
            entry(Opens::Link, Some("--roblox")).exec(&launcher),
            "/home/someone/Apps/Quadra.AppImage --roblox %u"
        );
    }

    #[test]
    fn paths_with_spaces_are_quoted() {
        let launcher = Launcher::Executable(PathBuf::from("/home/someone/My Apps/Quadra.AppImage"));
        assert_eq!(
            entry(Opens::File, Some("--studio")).exec(&launcher),
            "\"/home/someone/My Apps/Quadra.AppImage\" --studio %f"
        );
    }

    #[test]
    fn flatpak_entries_forward_opened_files() {
        let launcher = Launcher::Flatpak {
            app_id: "org.quadrahq.Quadra".to_owned(),
        };
        assert_eq!(
            entry(Opens::File, Some("--studio")).exec(&launcher),
            "flatpak run --file-forwarding org.quadrahq.Quadra --studio @@ %f @@"
        );
        assert_eq!(
            entry(Opens::Nothing, None).exec(&launcher),
            "flatpak run org.quadrahq.Quadra"
        );
    }
}
