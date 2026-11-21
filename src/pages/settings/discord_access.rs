use crate::backend;
use crate::components::command_dialog::command_dialog;
use crate::components::dialog::Alert;

pub fn dialog_if_needed() -> Option<Alert> {
    let missing = backend::missing_discord_access()?;
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

    Some(command_dialog(
        "Let Quadra reach Discord",
        &format!("{found} Run this in a terminal to allow it, then restart Quadra:"),
        &missing.command,
    ))
}
