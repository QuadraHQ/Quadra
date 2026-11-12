use crate::backend;
use crate::state::AppState;
use crate::utils::format_size;
use adw::prelude::*;
use gtk::glib;

const KEEP_RESPONSE: &str = "keep";
const DELETE_RESPONSE: &str = "delete";

pub fn show_legacy_cleanup_offers_from(state: &AppState, window: &adw::ApplicationWindow) {
    state.connect_legacy_cleanup_bytes_notify(glib::clone!(
        #[weak]
        window,
        move |state| {
            if state.legacy_cleanup_bytes() > 0 {
                offer(&window, state);
            }
        }
    ));
}

fn offer(window: &adw::ApplicationWindow, state: &AppState) {
    let dialog = adw::AlertDialog::builder()
        .heading("Delete the old Quadra?")
        .body(format!(
            "The previous version of Quadra is still on this computer and uses {}. \
             This Quadra doesn't need it. Deleting it also removes its settings, its Roblox \
             sign-in and the shortcuts that start it. Darling itself isn't touched.",
            format_size(state.legacy_cleanup_bytes())
        ))
        .build();
    dialog.add_response(KEEP_RESPONSE, "Keep it");
    dialog.add_response(DELETE_RESPONSE, "Delete it");
    dialog.set_response_appearance(DELETE_RESPONSE, adw::ResponseAppearance::Destructive);
    dialog.set_close_response(KEEP_RESPONSE);

    dialog.connect_response(
        Some(KEEP_RESPONSE),
        glib::clone!(
            #[strong]
            state,
            move |_, _| {
                state.set_legacy_cleanup_dismissed(true);
                state.set_legacy_cleanup_bytes(0);
            }
        ),
    );
    dialog.connect_response(
        Some(DELETE_RESPONSE),
        glib::clone!(
            #[strong]
            state,
            move |_, _| backend::delete_old_quadra(&state)
        ),
    );
    dialog.present(Some(window));
}
