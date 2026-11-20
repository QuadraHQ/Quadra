use super::privacy::privacy_group;
use crate::components::style::Look;
use crate::quadra::Message;
use crate::state::AppState;
use iced::Element;

pub fn network_section<'a>(state: &AppState, look: Look) -> Vec<Element<'a, Message>> {
    vec![privacy_group(state, look)]
}
