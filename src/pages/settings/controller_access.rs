use crate::backend;
use crate::components::command_dialog::show_command_dialog;
use crate::utils::is_running_in_flatpak;

pub fn controller_access_row() -> Option<adw::SwitchRow> {
    if !is_running_in_flatpak() {
        return None;
    }

    let row = adw::SwitchRow::builder()
        .title("Enable controller support")
        .subtitle("Quadra runs as a Flatpak, so it needs your permission to see controllers.")
        .active(backend::controller_access_is_granted())
        .build();

    row.connect_active_notify(|row| {
        if let Some(command) = backend::controller_access_command(row.is_active()) {
            show_controller_access_dialog(row, row.is_active(), &command);
        }
    });

    Some(row)
}

fn show_controller_access_dialog(row: &adw::SwitchRow, allow_controllers: bool, command: &str) {
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

    show_command_dialog(row, heading, body, command);
}
