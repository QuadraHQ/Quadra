use crate::backend::{self, MissingDiscordAccess};
use crate::components::command_dialog::show_command_dialog;

pub fn ask_for_discord_access_when_turned_on(row: &adw::SwitchRow) {
    row.connect_active_notify(|row| {
        if row.is_active()
            && let Some(missing) = backend::missing_discord_access()
        {
            show_discord_access_dialog(row, &missing);
        }
    });
}

fn show_discord_access_dialog(row: &adw::SwitchRow, missing: &MissingDiscordAccess) {
    let found = if missing.discords.is_empty() {
        "Quadra couldn't find Discord running, so this allows every Discord app it supports. \
         To allow only yours, open Discord and turn this on again."
            .to_owned()
    } else {
        format!(
            "Quadra found {} running, but runs as a Flatpak, so it can't reach it yet.",
            missing.discords.join(" and ")
        )
    };

    show_command_dialog(
        row,
        "Let Quadra reach Discord",
        &format!("{found} Run this in a terminal to allow it, then restart Quadra:"),
        &missing.command,
    );
}
