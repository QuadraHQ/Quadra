use super::dialog::{Alert, Response};

const LONGEST_PLAIN_MESSAGE: usize = 160;

pub fn error_dialog(message: &str) -> Alert {
    let is_long = message.len() > LONGEST_PLAIN_MESSAGE || message.trim().contains('\n');
    let alert = if is_long {
        Alert::new(
            "Something went wrong",
            "Here's what happened. You can select and copy it.",
        )
        .with_code(message.trim())
    } else {
        Alert::new("Something went wrong", message)
    };
    alert.with_response(Response::close("Close"))
}
