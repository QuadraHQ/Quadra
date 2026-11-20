use crate::backend;
use crate::components::command_dialog::command_dialog;
use crate::components::dialog::Alert;

pub fn dialog_if_needed() -> Option<Alert> {
    let command = backend::missing_vr_access()?;
    Some(command_dialog(
        "Let Quadra reach your headset",
        "Quadra runs as a Flatpak, so it can't reach this computer's OpenXR runtime yet. \
         Run this in a terminal to let it read where Monado or WiVRn is installed and \
         talk to it, then restart Quadra:",
        &command,
    ))
}
