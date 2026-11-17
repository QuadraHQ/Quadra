use super::dialog::{Alert, Response};

pub fn error_dialog(message: &str) -> Alert {
    Alert::new("Something went wrong", message).with_response(Response::close("Close"))
}
