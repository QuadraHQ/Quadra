use super::preferences::{body, push_button};
use super::style::{self, Emphasis, Look};
use crate::backend;
use crate::quadra::{Message, act};
use crate::state::AppState;
use iced::widget::{container, row};
use iced::{Alignment, Element, Length};

pub fn update_banner<'a>(state: &AppState, look: Look) -> Option<Element<'a, Message>> {
    let version = state.available_update();
    if version.is_empty() {
        return None;
    }

    let update_button = push_button(look, "Update", Emphasis::Suggested)
        .on_press_maybe((!state.busy()).then(|| act(backend::install_update)));
    let content = row![
        body(look, format!("Quadra {version} is available")).width(Length::Fill),
        update_button
    ]
    .spacing(12)
    .align_y(Alignment::Center);

    Some(
        container(content)
            .padding([8, 16])
            .width(Length::Fill)
            .style(style::banner(look))
            .into(),
    )
}
