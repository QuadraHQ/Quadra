use super::play_button::play_button;
use crate::components::icon::{Symbol, app_icon};
use crate::components::preferences::{caption, symbol_button, title};
use crate::components::roblox_status::status_text;
use crate::components::style::{Emphasis, Look, TITLE_SIZE};
use crate::quadra::Message;
use crate::state::{AppState, Client};
use crate::utils::format_playtime;
use iced::widget::{Column, center, space};
use iced::{Alignment, Element};

pub fn client_page<'a>(state: &AppState, look: Look, client: Client) -> Element<'a, Message> {
    let playtime = (client == Client::Player)
        .then(|| playtime_label(state, look))
        .flatten();

    let content = Column::new()
        .push(app_icon(128.0))
        .push(title(look, client.home_title(), TITLE_SIZE))
        .push(
            caption(look, status_text(state, client))
                .center()
                .size(look.sized(14.0)),
        )
        .push(playtime)
        .push(space().height(0))
        .push(
            symbol_button(look, Symbol::Terminal, "Show Logs", Emphasis::Flat)
                .on_press(Message::ShowLogs),
        )
        .push(play_button(state, look, client))
        .spacing(12)
        .align_x(Alignment::Center);

    center(content).into()
}

fn playtime_label<'a>(state: &AppState, look: Look) -> Option<iced::widget::Text<'a>> {
    let seconds = state.playtime_seconds();
    (state.show_playtime() && seconds > 0).then(|| {
        caption(look, format!("Played for {}", format_playtime(seconds))).size(look.sized(14.0))
    })
}
