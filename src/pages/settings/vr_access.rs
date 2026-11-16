use crate::backend;
use crate::components::command_dialog::show_command_dialog;

pub fn ask_for_vr_access_when_turned_on(row: &adw::SwitchRow) {
    row.connect_active_notify(|row| {
        if row.is_active()
            && let Some(command) = backend::missing_vr_access()
        {
            show_command_dialog(
                row,
                "Let Quadra reach your headset",
                "Quadra runs as a Flatpak, so it can't reach this computer's OpenXR runtime yet. \
                 Run this in a terminal to let it read where Monado or WiVRn is installed and \
                 talk to it, then restart Quadra:",
                &command,
            );
        }
    });
}
