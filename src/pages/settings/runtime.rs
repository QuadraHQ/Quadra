use super::Settings;
use super::rosemary_options::{graphics_group, input_group};
use super::rosemary_runtime::rosemary_runtime_group;
use crate::components::style::Look;
use crate::quadra::Message;
use crate::state::AppState;
use iced::Element;

pub fn runtime_section<'a>(
    settings: &'a Settings,
    state: &AppState,
    look: Look,
) -> Vec<Element<'a, Message>> {
    vec![
        rosemary_runtime_group(settings, state, look),
        graphics_group(settings, state, look),
        input_group(settings, state, look),
    ]
}
