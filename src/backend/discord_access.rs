use super::flatpak_override;
use crate::utils::is_running_in_flatpak;
use gtk::glib;
use std::fs;
use std::path::{Path, PathBuf};

const SOCKET_TABLE: &str = "/proc/net/unix";
const SOCKET_NAME_PREFIX: &str = "discord-ipc-";
const DEFAULT_SOCKET_NAME: &str = "discord-ipc-0";

struct KnownDiscord {
    name: &'static str,
    socket_directory: &'static str,
}

const KNOWN_DISCORDS: [KnownDiscord; 7] = [
    KnownDiscord {
        name: "Discord",
        socket_directory: "",
    },
    KnownDiscord {
        name: "Discord (Flatpak)",
        socket_directory: "app/com.discordapp.Discord",
    },
    KnownDiscord {
        name: "Discord (Flatpak)",
        socket_directory: ".flatpak/com.discordapp.Discord/xdg-run",
    },
    KnownDiscord {
        name: "Vesktop (Flatpak)",
        socket_directory: "app/dev.vencord.Vesktop",
    },
    KnownDiscord {
        name: "Vesktop (Flatpak)",
        socket_directory: ".flatpak/dev.vencord.Vesktop/xdg-run",
    },
    KnownDiscord {
        name: "Discord (Snap)",
        socket_directory: "snap.discord",
    },
    KnownDiscord {
        name: "Discord Canary (Snap)",
        socket_directory: "snap.discord-canary",
    },
];

#[derive(Debug, PartialEq, Eq)]
pub struct MissingDiscordAccess {
    pub discords: Vec<&'static str>,
    pub command: String,
}

pub fn missing() -> Option<MissingDiscordAccess> {
    if !is_running_in_flatpak() {
        return None;
    }
    let runtime_directory = glib::user_runtime_dir();
    let socket_table = fs::read_to_string(SOCKET_TABLE).unwrap_or_default();
    let flatpak_id = flatpak_override::quadra_id();
    missing_from(&socket_table, &runtime_directory, &flatpak_id, |path| {
        path.exists()
    })
}

fn missing_from(
    socket_table: &str,
    runtime_directory: &Path,
    flatpak_id: &str,
    is_visible: impl Fn(&Path) -> bool,
) -> Option<MissingDiscordAccess> {
    let running = running_discords(socket_table, runtime_directory);

    let can_reach_discord = if running.is_empty() {
        KNOWN_DISCORDS
            .iter()
            .filter(|discord| !discord.socket_directory.is_empty())
            .any(|discord| is_visible(&runtime_directory.join(discord.socket_directory)))
    } else {
        running
            .iter()
            .any(|(_, socket)| is_visible(&runtime_directory.join(socket)))
    };
    if can_reach_discord {
        return None;
    }

    let permissions: Vec<(&'static str, String)> = if running.is_empty() {
        KNOWN_DISCORDS
            .iter()
            .map(|discord| {
                let default_socket = Path::new(discord.socket_directory).join(DEFAULT_SOCKET_NAME);
                (discord.name, permission_for(discord, &default_socket))
            })
            .collect()
    } else {
        running
            .iter()
            .map(|(discord, socket)| (discord.name, permission_for(discord, socket)))
            .collect()
    };

    let mut discords = Vec::new();
    if !running.is_empty() {
        for (name, _) in &permissions {
            if !discords.contains(name) {
                discords.push(*name);
            }
        }
    }
    let command = flatpak_override::command(
        flatpak_id,
        permissions
            .iter()
            .map(|(_, permission)| permission.as_str()),
    );

    Some(MissingDiscordAccess { discords, command })
}

fn running_discords(
    socket_table: &str,
    runtime_directory: &Path,
) -> Vec<(&'static KnownDiscord, PathBuf)> {
    socket_table
        .lines()
        .filter_map(|line| line.split_whitespace().last())
        .filter_map(|path| Path::new(path).strip_prefix(runtime_directory).ok())
        .filter(|socket| {
            socket
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(SOCKET_NAME_PREFIX))
        })
        .filter_map(|socket| {
            let directory = socket.parent()?;
            let discord = KNOWN_DISCORDS
                .iter()
                .find(|discord| directory == Path::new(discord.socket_directory))?;
            Some((discord, socket.to_owned()))
        })
        .collect()
}

fn permission_for(discord: &KnownDiscord, socket: &Path) -> String {
    if discord.socket_directory.is_empty() {
        format!("xdg-run/{}", socket.display())
    } else {
        format!("xdg-run/{}:create", discord.socket_directory)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RUNTIME_DIRECTORY: &str = "/run/user/1000";
    const FLATPAK_DISCORD_TABLE: &str = "\
Num       RefCount Protocol Flags    Type St Inode Path
00000000d57099fc: 00000002 00000000 00010000 0001 01 37139 /run/user/1000/app/com.discordapp.Discord/discord-ipc-0
000000003c1a2b4d: 00000002 00000000 00010000 0001 01 41200 /run/user/1000/wayland-0
000000003c1a2b4e: 00000003 00000000 00000000 0001 03 41201
";

    fn missing_with(table: &str, visible: &[&str]) -> Option<MissingDiscordAccess> {
        missing_from(
            table,
            Path::new(RUNTIME_DIRECTORY),
            "org.quadrahq.Quadra",
            |path| visible.iter().any(|visible| path == Path::new(visible)),
        )
    }

    #[test]
    fn asks_only_for_the_running_discord() {
        assert_eq!(
            missing_with(FLATPAK_DISCORD_TABLE, &[]),
            Some(MissingDiscordAccess {
                discords: vec!["Discord (Flatpak)"],
                command: "flatpak override --user \
                          --filesystem=xdg-run/app/com.discordapp.Discord:create \
                          org.quadrahq.Quadra"
                    .to_owned(),
            })
        );
    }

    #[test]
    fn asks_for_the_exact_socket_of_a_native_discord() {
        let table = "00: 00000002 00000000 00010000 0001 01 1 /run/user/1000/discord-ipc-1\n";
        assert_eq!(
            missing_with(table, &[]).unwrap().command,
            "flatpak override --user --filesystem=xdg-run/discord-ipc-1 org.quadrahq.Quadra"
        );
    }

    #[test]
    fn asks_for_nothing_once_the_socket_is_visible() {
        assert_eq!(
            missing_with(
                FLATPAK_DISCORD_TABLE,
                &["/run/user/1000/app/com.discordapp.Discord/discord-ipc-0"]
            ),
            None
        );
    }

    #[test]
    fn ignores_sockets_of_unknown_apps() {
        let table = "00: 00000002 00000000 00010000 0001 01 1 /run/user/1000/app/com.example.Chat/discord-ipc-0\n";
        assert!(missing_with(table, &[]).unwrap().discords.is_empty());
    }

    #[test]
    fn asks_for_every_known_discord_when_none_is_running() {
        let missing = missing_with("", &[]).unwrap();
        assert!(missing.discords.is_empty());
        assert_eq!(
            missing.command,
            "flatpak override --user \
             --filesystem=xdg-run/discord-ipc-0 \
             --filesystem=xdg-run/app/com.discordapp.Discord:create \
             --filesystem=xdg-run/.flatpak/com.discordapp.Discord/xdg-run:create \
             --filesystem=xdg-run/app/dev.vencord.Vesktop:create \
             --filesystem=xdg-run/.flatpak/dev.vencord.Vesktop/xdg-run:create \
             --filesystem=xdg-run/snap.discord:create \
             --filesystem=xdg-run/snap.discord-canary:create \
             org.quadrahq.Quadra"
        );
    }

    #[test]
    fn asks_for_nothing_when_none_is_running_but_access_was_given() {
        assert_eq!(
            missing_with("", &["/run/user/1000/app/com.discordapp.Discord"]),
            None
        );
    }
}
