use crate::backend;
use crate::components::dialog::{Alert, Response};
use crate::components::preferences::{preference_group, preference_row, push_button};
use crate::components::style::{Emphasis, Look};
use crate::quadra::{Message, act};
use crate::state::AppState;
use iced::Element;

pub fn account_group<'a>(state: &AppState, look: Look) -> Element<'a, Message> {
    let signed_in = state.signed_in();
    let sign_out_button = push_button(look, "Sign out", Emphasis::Destructive)
        .on_press_maybe((signed_in && !state.running()).then(|| Message::Alert(sign_out_dialog())));

    preference_group(look)
        .title("Account")
        .push(
            preference_row(look, "Roblox account")
                .subtitle(if signed_in {
                    "Signed in. The session is stored only on this computer"
                } else {
                    "Not signed in"
                })
                .suffix(sign_out_button),
        )
        .into()
}

fn sign_out_dialog() -> Alert {
    Alert::new(
        "Sign out of Roblox?",
        "Quadra deletes the saved session. You'll sign in again the next time you play.",
    )
    .with_response(Response::close("Cancel"))
    .with_response(Response::destructive("Sign out", act(backend::sign_out)))
}
