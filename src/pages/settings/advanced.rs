use super::Settings;
use super::environment_variables::environment_variables_group;
use super::fast_flags::fast_flags_section;
use super::launch_wrapper::launch_wrapper_group;
use super::network::network_section;
use crate::components::icon::{self, Symbol};
use crate::components::preferences::{body, caption};
use crate::components::style::Look;
use crate::quadra::Message;
use crate::state::AppState;
use iced::widget::column;
use iced::{Alignment, Element, Length};

pub fn advanced_section<'a>(
    settings: &'a Settings,
    state: &AppState,
    look: Look,
) -> Vec<Element<'a, Message>> {
    let warning = column![
        icon::tinted(Symbol::Warning, look.warning),
        body(look, "Advanced settings"),
        caption(
            look,
            "These options can break Quadra or Roblox if misconfigured. \
             Only change them if you know what you're doing.",
        )
        .center(),
    ]
    .spacing(4)
    .width(Length::Fill)
    .align_x(Alignment::Center);

    let mut sections = vec![
        warning.into(),
        launch_wrapper_group(settings, state, look),
        environment_variables_group(&settings.variables, look),
    ];
    sections.extend(network_section(state, look));
    sections.extend(fast_flags_section(settings, state, look));
    sections
}
