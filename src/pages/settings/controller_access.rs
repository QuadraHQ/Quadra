use super::SettingsMessage;
use super::bound_rows::switch;
use crate::backend;
use crate::components::command_dialog::command_dialog;
use crate::components::dialog::Alert;
use crate::components::preferences::{PreferenceRow, preference_row};
use crate::components::style::Look;
use crate::quadra::Message;
use crate::utils::is_running_in_flatpak;

pub fn controller_access_row<'a>(
    allows_controllers: bool,
    look: Look,
) -> Option<PreferenceRow<'a>> {
    is_running_in_flatpak().then(|| {
        preference_row(look, "Enable controller support")
            .subtitle("Quadra runs as a Flatpak, so it needs your permission to see controllers.")
            .suffix(switch(look, allows_controllers, true, |allow| {
                Message::Settings(SettingsMessage::ControllerAccessToggled(allow))
            }))
    })
}

pub fn dialog_if_needed(allow_controllers: bool) -> Option<Alert> {
    let command = backend::controller_access_command(allow_controllers)?;
    let (heading, body) = if allow_controllers {
        (
            "Let Quadra see controllers",
            "Run this in a terminal to let Quadra and Roblox use your controllers, then restart Quadra:",
        )
    } else {
        (
            "Stop Quadra from seeing controllers",
            "Run this in a terminal to take away Quadra's access to your devices, then restart Quadra:",
        )
    };
    Some(command_dialog(heading, body, &command))
}
