use super::SettingsMessage;
use super::bound_rows::switch;
use crate::backend;
use crate::components::dialog::{Alert, Response};
use crate::components::preferences::{PreferenceRow, preference_row};
use crate::components::style::Look;
use crate::quadra::Message;
use crate::state::AppState;
use crate::utils::is_running_in_flatpak;

const FLATPAK_INSTALL_COMMAND: &str =
    "flatpak install flathub org.freedesktop.Platform.VulkanLayer.MangoHud//26.08";

pub fn mangohud_row<'a>(state: &AppState, look: Look) -> PreferenceRow<'a> {
    preference_row(look, "MangoHud")
        .subtitle("Show an overlay with the frame rate, frame times and GPU usage")
        .suffix(switch(look, state.mangohud(), true, |on| {
            Message::Settings(SettingsMessage::MangoHudToggled(on))
        }))
}

pub fn dialog_if_needed() -> Option<Alert> {
    let flatpak_extension_is_missing = is_running_in_flatpak() && !backend::mangohud_is_installed();
    flatpak_extension_is_missing.then(|| {
        Alert::new(
            "Install MangoHud for Flatpak",
            format!(
                "Quadra runs as a Flatpak, so it can't use the MangoHud installed on your system. \
                 Install the MangoHud Flatpak extension, then restart Quadra:\n\n{FLATPAK_INSTALL_COMMAND}"
            ),
        )
        .with_response(Response::close("Close"))
        .with_response(Response::suggested(
            "Copy command",
            Message::CopyToClipboard(FLATPAK_INSTALL_COMMAND.to_owned()),
        ))
    })
}
