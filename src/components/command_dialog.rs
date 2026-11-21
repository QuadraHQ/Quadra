use super::dialog::{Alert, Response};
use crate::quadra::Message;

pub fn command_dialog(heading: &str, body: &str, command: &str) -> Alert {
    Alert::new(heading, body)
        .with_code(command)
        .with_response(Response::close("Close"))
        .with_response(Response::suggested(
            "Copy command",
            Message::CopyToClipboard(command.to_owned()),
        ))
}
