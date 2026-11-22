mod client_page;
mod play_button;

use crate::components::style::{self, Look};
use crate::quadra::{Message, act};
use crate::state::{AppState, Client};
use client_page::client_page;
use iced::widget::{button, column, container, row, text};
use iced::{Element, Length};

pub fn home_page<'a>(state: &AppState, look: Look) -> Element<'a, Message> {
    let selected = state.selected_client();
    let switcher = row(Client::ALL.map(|client| {
        button(
            text(client.short_name())
                .font(look.font)
                .size(look.sized(14.0)),
        )
        .padding([5, 18])
        .style(style::tab(look, client == selected))
        .on_press(act(move |state| state.set_selected_client(client)))
        .into()
    }))
    .spacing(4);

    column![
        container(container(switcher).padding(3).style(style::card(look)))
            .center_x(Length::Fill)
            .padding(iced::padding::top(12)),
        client_page(state, look, selected),
    ]
    .into()
}
