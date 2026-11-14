use super::desktop_entries::{DesktopEntry, Opens};
use super::{mime_defaults, paths};
use anyhow::{Context, ensure};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const PLACE_MIME_TYPES_FILE: &str = "org.quadrahq.Quadra.xml";
const PLACE_MIME_TYPES: &str = include_str!("../../data/org.quadrahq.Quadra.xml");

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Handler {
    RobloxLinks,
    StudioLinks,
    StudioPlaces,
}

impl Handler {
    pub const ALL: [Self; 3] = [Self::RobloxLinks, Self::StudioLinks, Self::StudioPlaces];

    pub fn title(self) -> &'static str {
        match self {
            Self::RobloxLinks => "Open Roblox links with Quadra",
            Self::StudioLinks => "Open Roblox Studio links with Quadra",
            Self::StudioPlaces => "Open place files with Quadra",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::RobloxLinks => {
                "roblox:// and roblox-player: links, like Play buttons on the website"
            }
            Self::StudioLinks => {
                "roblox-studio: and roblox-studio-auth: links, like Edit in Studio and signing in"
            }
            Self::StudioPlaces => ".rbxl and .rbxlx files open in Roblox Studio",
        }
    }

    pub(super) fn entry(self) -> DesktopEntry {
        match self {
            Self::RobloxLinks => DesktopEntry {
                file_name: "org.quadrahq.Quadra.RobloxLinks.desktop",
                name: "Roblox",
                description: "Opens Roblox links with Quadra",
                command_line_option: Some("--roblox"),
                opens: Opens::Link,
                mime_types: &["x-scheme-handler/roblox", "x-scheme-handler/roblox-player"],
                shown_in_app_menu: false,
            },
            Self::StudioLinks => DesktopEntry {
                file_name: "org.quadrahq.Quadra.StudioLinks.desktop",
                name: "Roblox Studio",
                description: "Opens Roblox Studio links with Quadra",
                command_line_option: Some("--studio"),
                opens: Opens::Link,
                mime_types: &[
                    "x-scheme-handler/roblox-studio",
                    "x-scheme-handler/roblox-studio-auth",
                ],
                shown_in_app_menu: false,
            },
            Self::StudioPlaces => DesktopEntry {
                file_name: "org.quadrahq.Quadra.StudioPlaces.desktop",
                name: "Roblox Studio",
                description: "Opens Roblox places in Roblox Studio with Quadra",
                command_line_option: Some("--studio"),
                opens: Opens::File,
                mime_types: &["application/x-roblox-rbxl", "application/x-roblox-rbxlx"],
                shown_in_app_menu: false,
            },
        }
    }
}

pub fn is_enabled(handler: Handler) -> bool {
    let entry = handler.entry();
    entry.exists() && mime_defaults::is_default_for_all(entry.file_name, entry.mime_types)
}

pub fn enable(handler: Handler) -> anyhow::Result<()> {
    if handler == Handler::StudioPlaces {
        install_place_mime_types()?;
    }
    let entry = handler.entry();
    entry.add()?;
    mime_defaults::make_default(entry.file_name, entry.mime_types)
}

pub fn disable(handler: Handler) -> anyhow::Result<()> {
    let entry = handler.entry();
    mime_defaults::stop_being_default(entry.file_name, entry.mime_types)?;
    entry.remove()?;
    if handler == Handler::StudioPlaces {
        remove_place_mime_types()?;
    }
    Ok(())
}

fn place_mime_types_path() -> PathBuf {
    paths::mime_directory()
        .join("packages")
        .join(PLACE_MIME_TYPES_FILE)
}

fn install_place_mime_types() -> anyhow::Result<()> {
    let path = place_mime_types_path();
    if let Some(directory) = path.parent() {
        fs::create_dir_all(directory)?;
    }
    fs::write(&path, PLACE_MIME_TYPES)
        .with_context(|| format!("Couldn't add the place file types to {}", path.display()))?;
    update_mime_database()
}

fn remove_place_mime_types() -> anyhow::Result<()> {
    let path = place_mime_types_path();
    if path.exists() {
        fs::remove_file(&path).with_context(|| format!("Couldn't remove {}", path.display()))?;
        update_mime_database()?;
    }
    Ok(())
}

fn update_mime_database() -> anyhow::Result<()> {
    let status = Command::new("update-mime-database")
        .arg(paths::mime_directory())
        .status()
        .context("Couldn't run update-mime-database to register place files")?;
    ensure!(
        status.success(),
        "update-mime-database couldn't register place files"
    );
    Ok(())
}
