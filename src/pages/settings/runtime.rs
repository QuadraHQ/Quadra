use super::Settings;
use super::bound_rows::choice_row;
use super::rosemary_options::{graphics_group, input_group};
use super::rosemary_runtime::rosemary_runtime_group;
use crate::components::preferences::preference_group;
use crate::components::style::Look;
use crate::quadra::Message;
use crate::state::AppState;
use iced::Element;

pub fn runtime_section<'a>(
    settings: &'a Settings,
    state: &AppState,
    look: Look,
) -> Vec<Element<'a, Message>> {
    let runtime_group = preference_group(look)
        .title("Runtime")
        .description("Which compatibility layer the macOS Roblox client runs on.")
        .push(choice_row(
            look,
            "Run Roblox with",
            state.runtime(),
            |state, runtime| state.set_runtime(runtime),
        ));

    vec![
        runtime_group.into(),
        rosemary_runtime_group(settings, state, look),
        graphics_group(settings, state, look),
        input_group(settings, state, look),
    ]
}
