use super::bound_rows::switch_row;
use crate::backend;
use crate::state::AppState;
use crate::utils::is_running_in_flatpak;
use adw::prelude::*;

const FLATPAK_INSTALL_COMMAND: &str =
    "flatpak install flathub org.freedesktop.Platform.VulkanLayer.MangoHud//26.08";
const COPY_RESPONSE: &str = "copy";
const CLOSE_RESPONSE: &str = "close";

pub fn mangohud_row(state: &AppState) -> adw::SwitchRow {
    let row = switch_row(state, "mangohud", "MangoHud");
    row.set_subtitle("Show an overlay with the frame rate, frame times and GPU usage");

    row.connect_active_notify(|row| {
        let flatpak_extension_is_missing =
            is_running_in_flatpak() && !backend::mangohud_is_installed();
        if row.is_active() && flatpak_extension_is_missing {
            show_flatpak_install_dialog(row);
        }
    });

    row
}

fn show_flatpak_install_dialog(row: &adw::SwitchRow) {
    let dialog = adw::AlertDialog::builder()
        .heading("Install MangoHud for Flatpak")
        .body(format!(
            "Quadra runs as a Flatpak, so it can't use the MangoHud installed on your system. \
             Install the MangoHud Flatpak extension, then restart Quadra:\n\n{FLATPAK_INSTALL_COMMAND}"
        ))
        .build();
    dialog.add_response(CLOSE_RESPONSE, "Close");
    dialog.add_response(COPY_RESPONSE, "Copy command");
    dialog.set_response_appearance(COPY_RESPONSE, adw::ResponseAppearance::Suggested);
    dialog.set_default_response(Some(COPY_RESPONSE));
    dialog.set_close_response(CLOSE_RESPONSE);

    dialog.connect_response(Some(COPY_RESPONSE), |dialog, _| {
        dialog.clipboard().set_text(FLATPAK_INSTALL_COMMAND);
    });

    dialog.present(Some(row));
}
