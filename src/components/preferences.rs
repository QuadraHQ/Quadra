use super::icon::{self, Symbol};
use super::style::{self, BODY_SIZE, CAPTION_SIZE, Emphasis, HEADING_SIZE, Look};
use crate::quadra::Message;
use iced::widget::text::{IntoFragment, Wrapping};
use iced::widget::{
    Button, Column, Text, button, column, container, row, rule, space, text, tooltip,
};
use iced::{Alignment, Element, Length, Padding};
use std::cell::{Cell, RefCell};

const ROW_MIN_HEIGHT: f32 = 50.0;
const ROW_PADDING: Padding = Padding {
    top: 8.0,
    right: 12.0,
    bottom: 8.0,
    left: 14.0,
};
const GROUP_SPACING: f32 = 6.0;
pub const MAXIMUM_CONTENT_WIDTH: f32 = 640.0;

thread_local! {
    static SEARCHED_WORDS: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
    static LAST_ROW_FOUND: Cell<Option<bool>> = const { Cell::new(None) };
    static ANYTHING_FOUND: Cell<bool> = const { Cell::new(false) };
}

pub fn searching<T>(query: &str, build: impl FnOnce() -> T) -> (T, bool) {
    let words = query.split_whitespace().map(str::to_lowercase).collect();
    SEARCHED_WORDS.set(Some(words));
    ANYTHING_FOUND.set(false);
    let built = build();
    SEARCHED_WORDS.set(None);
    LAST_ROW_FOUND.set(None);
    (built, ANYTHING_FOUND.take())
}

fn is_searching() -> bool {
    SEARCHED_WORDS.with_borrow(Option::is_some)
}

fn is_found_in(text: &str) -> bool {
    let text = text.to_lowercase();
    SEARCHED_WORDS.with_borrow(|words| {
        words
            .as_ref()
            .is_none_or(|words| words.iter().all(|word| text.contains(word.as_str())))
    })
}

pub fn searchable<'a>(
    content: impl Into<Element<'a, Message>>,
    words: &str,
) -> Element<'a, Message> {
    if is_searching() {
        LAST_ROW_FOUND.set(Some(is_found_in(words)));
    }
    content.into()
}

pub fn body<'a>(look: Look, content: impl IntoFragment<'a>) -> Text<'a> {
    text(content)
        .font(look.font)
        .size(BODY_SIZE)
        .wrapping(Wrapping::WordOrGlyph)
}

pub fn caption<'a>(look: Look, content: impl IntoFragment<'a>) -> Text<'a> {
    text(content)
        .font(look.font)
        .size(CAPTION_SIZE)
        .wrapping(Wrapping::WordOrGlyph)
        .style(style::dimmed(look))
}

pub fn heading<'a>(look: Look, content: impl IntoFragment<'a>) -> Text<'a> {
    text(content)
        .font(iced::Font {
            weight: iced::font::Weight::Bold,
            ..look.font
        })
        .size(HEADING_SIZE)
        .style(style::dimmed(look))
}

pub fn title<'a>(look: Look, content: impl IntoFragment<'a>, size: f32) -> Text<'a> {
    text(content).size(size).font(iced::Font {
        weight: iced::font::Weight::Bold,
        ..look.font
    })
}

pub fn push_button<'a>(
    look: Look,
    label: impl IntoFragment<'a>,
    emphasis: Emphasis,
) -> Button<'a, Message> {
    button(text(label).font(look.font).size(BODY_SIZE))
        .padding([6, 14])
        .style(style::emphasized(look, emphasis))
}

pub fn symbol_button<'a>(
    look: Look,
    symbol: Symbol,
    label: impl IntoFragment<'a>,
    emphasis: Emphasis,
) -> Button<'a, Message> {
    let color = match emphasis {
        Emphasis::Suggested => look.on_accent,
        Emphasis::Destructive => look.destructive,
        Emphasis::Normal | Emphasis::Flat => look.text,
    };
    let content = row![
        icon::tinted(symbol, color),
        text(label).font(look.font).size(BODY_SIZE)
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    button(content)
        .padding([6, 14])
        .style(style::emphasized(look, emphasis))
}

pub fn icon_button<'a>(
    look: Look,
    symbol: Symbol,
    explanation: &'a str,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    let button = button(icon::symbol(symbol, look))
        .padding(7)
        .style(style::emphasized(look, Emphasis::Flat))
        .on_press_maybe(on_press);
    with_tooltip(look, button, explanation)
}

pub fn with_tooltip<'a>(
    look: Look,
    content: impl Into<Element<'a, Message>>,
    explanation: &'a str,
) -> Element<'a, Message> {
    tooltip(
        content,
        container(caption(look, explanation).style(move |_| text::Style {
            color: Some(look.text),
        }))
        .padding([4, 8])
        .style(style::tooltip(look)),
        tooltip::Position::Top,
    )
    .gap(4)
    .into()
}

pub struct PreferenceRow<'a> {
    look: Look,
    words: String,
    title: Text<'a>,
    subtitle: Option<Text<'a>>,
    prefix: Option<Element<'a, Message>>,
    suffixes: Vec<Element<'a, Message>>,
    enabled: bool,
}

pub fn preference_row<'a>(look: Look, title: impl IntoFragment<'a>) -> PreferenceRow<'a> {
    let title = title.into_fragment();
    PreferenceRow {
        look,
        words: title.to_string(),
        title: body(look, title),
        subtitle: None,
        prefix: None,
        suffixes: Vec::new(),
        enabled: true,
    }
}

impl<'a> PreferenceRow<'a> {
    pub fn subtitle(mut self, subtitle: impl IntoFragment<'a>) -> Self {
        let subtitle = subtitle.into_fragment();
        self.words.push(' ');
        self.words.push_str(&subtitle);
        self.subtitle = Some(caption(self.look, subtitle));
        self
    }

    pub fn subtitle_maybe(self, subtitle: Option<impl IntoFragment<'a>>) -> Self {
        match subtitle {
            Some(subtitle) => self.subtitle(subtitle),
            None => self,
        }
    }

    pub fn monospace_subtitle(mut self, subtitle: impl IntoFragment<'a>) -> Self {
        let subtitle = subtitle.into_fragment();
        self.words.push(' ');
        self.words.push_str(&subtitle);
        self.subtitle = Some(caption(self.look, subtitle).font(self.look.mono).size(11));
        self
    }

    pub fn prefix(mut self, prefix: impl Into<Element<'a, Message>>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    pub fn suffix(mut self, suffix: impl Into<Element<'a, Message>>) -> Self {
        self.suffixes.push(suffix.into());
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

impl<'a> From<PreferenceRow<'a>> for Element<'a, Message> {
    fn from(preference: PreferenceRow<'a>) -> Self {
        if is_searching() {
            LAST_ROW_FOUND.set(Some(is_found_in(&preference.words)));
        }
        let look = preference.look;
        let dimmed_if_disabled = move |_: &iced::Theme| text::Style {
            color: Some(if preference.enabled {
                look.text
            } else {
                look.dimmed
            }),
        };
        let labels = Column::new()
            .push(preference.title.style(dimmed_if_disabled))
            .push(preference.subtitle)
            .spacing(2)
            .width(Length::Fill);

        let mut content = row![].spacing(10).align_y(Alignment::Center);
        if let Some(prefix) = preference.prefix {
            content = content.push(prefix);
        }
        content = content.push(labels);
        for suffix in preference.suffixes {
            content = content.push(suffix);
        }

        container(content)
            .padding(ROW_PADDING)
            .width(Length::Fill)
            .height(Length::Shrink)
            .center_y(Length::Shrink)
            .style(move |_| container::Style::default())
            .into()
    }
}

pub fn row_height_floor<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    row![space().height(ROW_MIN_HEIGHT), content.into()]
        .align_y(Alignment::Center)
        .into()
}

pub struct PreferenceGroup<'a> {
    look: Look,
    title_is_found: bool,
    title: Option<Text<'a>>,
    description: Option<Text<'a>>,
    header_suffix: Option<Element<'a, Message>>,
    rows: Vec<Element<'a, Message>>,
}

pub fn preference_group<'a>(look: Look) -> PreferenceGroup<'a> {
    PreferenceGroup {
        look,
        title_is_found: false,
        title: None,
        description: None,
        header_suffix: None,
        rows: Vec::new(),
    }
}

impl<'a> PreferenceGroup<'a> {
    pub fn title(mut self, title: impl IntoFragment<'a>) -> Self {
        let title = title.into_fragment();
        self.title_is_found = is_searching() && is_found_in(&title);
        self.title = Some(heading(self.look, title));
        self
    }

    pub fn description(mut self, description: impl IntoFragment<'a>) -> Self {
        self.description = Some(caption(self.look, description));
        self
    }

    pub fn header_suffix(mut self, suffix: impl Into<Element<'a, Message>>) -> Self {
        self.header_suffix = Some(suffix.into());
        self
    }

    pub fn push(mut self, row: impl Into<Element<'a, Message>>) -> Self {
        let row = row.into();
        let is_found = LAST_ROW_FOUND.take() == Some(true);
        if !is_searching() || self.title_is_found || is_found {
            ANYTHING_FOUND.set(true);
            self.rows.push(row_height_floor(row));
        }
        self
    }

    pub fn push_maybe(self, row: Option<impl Into<Element<'a, Message>>>) -> Self {
        match row {
            Some(row) => self.push(row),
            None => self,
        }
    }

    pub fn extend(self, rows: impl IntoIterator<Item = impl Into<Element<'a, Message>>>) -> Self {
        rows.into_iter().fold(self, Self::push)
    }
}

impl<'a> From<PreferenceGroup<'a>> for Element<'a, Message> {
    fn from(group: PreferenceGroup<'a>) -> Self {
        if is_searching() && group.rows.is_empty() {
            return Element::from(None::<Element<'a, Message>>);
        }
        let look = group.look;
        let has_header = group.title.is_some() || group.description.is_some();
        let labels = column![]
            .push(group.title)
            .push(group.description)
            .spacing(4)
            .width(Length::Fill);
        let header = row![labels]
            .push(group.header_suffix)
            .spacing(8)
            .align_y(Alignment::End);

        let mut rows = Column::new();
        let count = group.rows.len();
        for (position, preference) in group.rows.into_iter().enumerate() {
            rows = rows.push(preference);
            if position + 1 < count {
                rows = rows.push(rule::horizontal(1).style(style::separator(look)));
            }
        }

        let mut group_column = Column::new().spacing(GROUP_SPACING + 4.0);
        if has_header {
            group_column = group_column.push(header);
        }
        if count > 0 {
            group_column = group_column.push(container(rows).style(style::card(look)));
        }
        group_column.into()
    }
}

pub fn page<'a>(sections: impl IntoIterator<Item = Element<'a, Message>>) -> Element<'a, Message> {
    let content = sections
        .into_iter()
        .fold(Column::new(), Column::push)
        .spacing(24)
        .padding(Padding::from([18, 16]))
        .max_width(MAXIMUM_CONTENT_WIDTH);
    container(content).center_x(Length::Fill).into()
}
