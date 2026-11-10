use adw::prelude::*;

const KEEP_PLAYING_RESPONSE: &str = "keep-playing";
const CLOSE_RESPONSE: &str = "close";

pub fn confirm_closing_roblox(parent: &impl IsA<gtk::Widget>, on_confirmed: impl Fn() + 'static) {
    let dialog = adw::AlertDialog::builder()
        .heading("Close Roblox too?")
        .body("Roblox is still running. Quitting Quadra closes Roblox as well.")
        .build();
    dialog.add_response(KEEP_PLAYING_RESPONSE, "Keep playing");
    dialog.add_response(CLOSE_RESPONSE, "Close Roblox");
    dialog.set_response_appearance(CLOSE_RESPONSE, adw::ResponseAppearance::Destructive);
    dialog.set_close_response(KEEP_PLAYING_RESPONSE);
    dialog.connect_response(Some(CLOSE_RESPONSE), move |_, _| on_confirmed());
    dialog.present(Some(parent));
}
