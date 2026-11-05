use crate::state::AppState;
use adw::prelude::*;
use gtk::glib;

pub fn show_errors_from(state: &AppState, window: &adw::ApplicationWindow) {
    state.connect_error_message_notify(glib::clone!(
        #[weak]
        window,
        move |state| {
            let dialog =
                adw::AlertDialog::new(Some("Something went wrong"), Some(&state.error_message()));
            dialog.add_response("close", "Close");
            dialog.present(Some(&window));
        }
    ));
}
