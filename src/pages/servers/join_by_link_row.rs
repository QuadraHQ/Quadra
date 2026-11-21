use super::ServersMessage;
use crate::components::icon::Symbol;
use crate::components::preferences::icon_button;
use crate::components::style::{self, Look};
use crate::quadra::Message;
use iced::widget::{container, row, text_input};
use iced::{Alignment, Element};

pub fn join_by_link_row<'a>(
    link: &'a str,
    is_invalid: bool,
    can_join: bool,
    look: Look,
) -> Element<'a, Message> {
    let entry = text_input("Join a game by ID or roblox.com link", link)
        .font(look.font)
        .size(14)
        .padding([7, 10])
        .style(style::input(look, !is_invalid))
        .on_input_maybe(
            can_join.then_some(|link| Message::Servers(ServersMessage::LinkChanged(link))),
        )
        .on_submit_maybe(can_join.then_some(Message::Servers(ServersMessage::JoinByLink)));

    let apply = icon_button(
        look,
        Symbol::Check,
        "Join",
        (can_join && !link.trim().is_empty())
            .then_some(Message::Servers(ServersMessage::JoinByLink)),
    );

    container(row![entry, apply].spacing(6).align_y(Alignment::Center))
        .padding([6, 12])
        .into()
}
