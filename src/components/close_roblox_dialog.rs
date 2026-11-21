use super::dialog::{Alert, Response};
use crate::quadra::Message;

pub fn confirm_closing_roblox() -> Alert {
    Alert::new(
        "Close Roblox too?",
        "Roblox is still running. Quitting Quadra closes Roblox as well.",
    )
    .with_response(Response::close("Keep playing"))
    .with_response(Response::destructive(
        "Close Roblox",
        Message::QuitClosingRoblox,
    ))
}
