use super::paths;
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

impl DesktopEntry {
    fn path(&self) -> PathBuf {
        paths::applications_directory().join(self.file_name)
    }

    pub fn exists(&self) -> bool {
        self.path().is_file()
    }

    pub fn add(&self) -> anyhow::Result<()> {
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
        fs::write(&path, self.contents(&icon)?)
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

    fn contents(&self, icon: &Path) -> anyhow::Result<String> {
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
            self.exec()?,
            icon.display()
        );
        if !self.mime_types.is_empty() {
            contents.push_str(&format!("MimeType={};\n", self.mime_types.join(";")));
        }
        if !self.shown_in_app_menu {
            contents.push_str("NoDisplay=true\n");
        }
        Ok(contents)
    }

    fn exec(&self) -> anyhow::Result<String> {
        let mut exec = if is_running_in_flatpak() {
            let app_id =
                std::env::var("FLATPAK_ID").context("Couldn't find Quadra's Flatpak ID")?;
            if self.opens == Opens::File {
                format!("flatpak run --file-forwarding {app_id}")
            } else {
                format!("flatpak run {app_id}")
            }
        } else {
            let executable =
                std::env::current_exe().context("Couldn't find Quadra's executable")?;
            quoted_for_exec(&executable.display().to_string())
        };

        if let Some(option) = self.command_line_option {
            exec.push(' ');
            exec.push_str(option);
        }

        match self.opens {
            Opens::Nothing => {}
            Opens::Link => exec.push_str(" %u"),
            Opens::File if is_running_in_flatpak() => exec.push_str(" @@ %f @@"),
            Opens::File => exec.push_str(" %f"),
        }

        Ok(exec)
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
