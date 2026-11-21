use super::icon::{self, Symbol};
use super::preferences::preference_row;
use super::style::Look;
use crate::quadra::{Message, act};
use crate::state::{AppState, Picker};
use iced::widget::{button, column, container, row, space, text};
use iced::{Alignment, Background, Border, Element, Length};

const CARD_SPACING: f32 = 10.0;
const LABEL_HEIGHT: f32 = 30.0;

pub struct Opener<'a> {
    pub picker: Picker,
    pub title: &'a str,
    pub subtitle: &'a str,
    pub shown: Element<'a, Message>,
}

pub fn picker_row<'a>(state: &AppState, look: Look, opener: Opener<'a>) -> Element<'a, Message> {
    let picker = opener.picker;
    let is_open = state.open_picker() == picker;
    let toggle = button(
        row![
            opener.shown,
            icon::tinted(
                if is_open {
                    Symbol::Collapse
                } else {
                    Symbol::Expand
                },
                look.dimmed,
            ),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    )
    .padding([5, 10])
    .on_press(act(move |state| {
        state.set_open_picker(if is_open { Picker::None } else { picker });
    }))
    .style(move |_, status| button::Style {
        background: Some(Background::Color(match status {
            button::Status::Hovered | button::Status::Pressed => look.hovered,
            button::Status::Active | button::Status::Disabled => look.raised,
        })),
        border: Border {
            radius: look.radius.into(),
            ..Border::default()
        },
        text_color: look.text,
        ..button::Style::default()
    });
    preference_row(look, opener.title)
        .subtitle(opener.subtitle)
        .suffix(toggle)
        .into()
}

pub fn picker_grid<'a>(
    cards: impl IntoIterator<Item = Element<'a, Message>>,
    per_row: usize,
) -> Element<'a, Message> {
    let mut rows = column![].spacing(CARD_SPACING);
    let mut cards = cards.into_iter().peekable();
    while cards.peek().is_some() {
        let mut grid_row = row![].spacing(CARD_SPACING);
        for _ in 0..per_row {
            grid_row = grid_row.push(match cards.next() {
                Some(card) => card,
                None => space().width(Length::Fill).into(),
            });
        }
        rows = rows.push(grid_row);
    }
    container(rows).padding(12).width(Length::Fill).into()
}

pub struct Card<'a> {
    pub preview: Element<'a, Message>,
    pub preview_height: f32,
    pub preview_background: Background,
    pub label: String,
    pub is_chosen: bool,
    pub on_press: Message,
    pub trailing: Option<Element<'a, Message>>,
}

pub fn picker_card<'a>(look: Look, card: Card<'a>) -> Element<'a, Message> {
    let preview_background = card.preview_background;
    let preview = container(card.preview)
        .height(card.preview_height)
        .width(Length::Fill)
        .clip(true)
        .style(move |_| container::Style {
            background: Some(preview_background),
            border: Border {
                radius: (look.radius - 2.0).max(0.0).into(),
                ..Border::default()
            },
            ..container::Style::default()
        });
    let mut label = row![
        text(card.label)
            .font(look.font)
            .size(13)
            .width(Length::Fill)
            .wrapping(text::Wrapping::None)
    ]
    .align_y(Alignment::Center)
    .height(LABEL_HEIGHT);
    if let Some(trailing) = card.trailing {
        label = label.push(trailing);
    } else if card.is_chosen {
        label = label.push(icon::tinted(Symbol::Check, look.accent_text));
    }
    button(column![preview, container(label).padding([2, 6])])
        .padding(4)
        .width(Length::Fill)
        .on_press(card.on_press)
        .style(card_button(look, card.is_chosen))
        .into()
}

pub fn adding_card<'a>(
    look: Look,
    caption: &'a str,
    height: f32,
    on_press: Message,
) -> Element<'a, Message> {
    let content = column![
        icon::tinted(Symbol::Add, look.dimmed),
        text(caption).font(look.font).size(13).color(look.dimmed),
    ]
    .spacing(6)
    .align_x(Alignment::Center);
    button(
        container(content)
            .center_x(Length::Fill)
            .center_y(height + LABEL_HEIGHT + 4.0),
    )
    .padding(4)
    .width(Length::Fill)
    .on_press(on_press)
    .style(card_button(look, false))
    .into()
}

pub fn card_button(
    look: Look,
    is_chosen: bool,
) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_, status| button::Style {
        background: Some(Background::Color(match status {
            button::Status::Hovered | button::Status::Pressed => look.hovered,
            button::Status::Active | button::Status::Disabled => look.raised,
        })),
        border: Border {
            color: if is_chosen { look.accent } else { look.border },
            width: if is_chosen { 2.0 } else { 1.0 },
            radius: look.radius.into(),
        },
        text_color: look.text,
        ..button::Style::default()
    }
}

pub fn choosing(chosen: impl Fn(&AppState) + Send + Sync + 'static) -> Message {
    act(move |state| {
        chosen(state);
        state.set_open_picker(Picker::None);
    })
}
