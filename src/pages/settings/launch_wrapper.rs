use super::{Settings, SettingsMessage};
use crate::components::preferences::preference_group;
use crate::components::style::{self, Look};
use crate::quadra::Message;
use crate::state::AppState;
use iced::Element;
use iced::widget::{container, text_input};

const DESCRIPTION: &str =
    "Programs that start Rosemary, like gamemoderun or gamescope -f --. They can be chained.";

pub fn launch_wrapper_group<'a>(
    settings: &'a Settings,
    state: &AppState,
    look: Look,
) -> Element<'a, Message> {
    let problem = settings.launch_wrapper_problem.as_deref();
    let input = text_input("Launch wrapper", &state.launch_wrapper())
        .font(look.mono)
        .size(look.sized(13.0))
        .padding([7, 10])
        .style(style::input(look, problem.is_none()))
        .on_input(|wrapper| Message::Settings(SettingsMessage::LaunchWrapperChanged(wrapper)));

    preference_group(look)
        .title("Launch wrapper")
        .description(problem.unwrap_or(DESCRIPTION))
        .push(container(input).padding([6, 12]))
        .into()
}
