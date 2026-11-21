use crate::components::icon::Symbol;
use crate::components::preferences::{PreferenceRow, icon_button, preference_row};
use crate::components::style::{self, Look};
use crate::quadra::{Message, act};
use crate::state::{AppState, Choice};
use iced::widget::{pick_list, row, text, toggler};
use iced::{Alignment, Element};
use std::fmt;
use std::ops::RangeInclusive;

const NOTHING_CHOSEN_LABEL: &str = "None";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Labeled<T>(pub T);

impl<T: Choice> fmt::Display for Labeled<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0.label())
    }
}

pub fn switch<'a>(
    look: Look,
    value: bool,
    enabled: bool,
    on_toggle: impl Fn(bool) -> Message + 'a,
) -> Element<'a, Message> {
    toggler(value)
        .size(22)
        .style(style::switch(look))
        .on_toggle_maybe(enabled.then_some(on_toggle))
        .into()
}

pub fn switch_row<'a>(
    look: Look,
    title: &'a str,
    value: bool,
    set: fn(&AppState, bool),
) -> PreferenceRow<'a> {
    switch_row_when(look, title, value, set, true)
}

pub fn switch_row_when<'a>(
    look: Look,
    title: &'a str,
    value: bool,
    set: fn(&AppState, bool),
    enabled: bool,
) -> PreferenceRow<'a> {
    preference_row(look, title)
        .suffix(switch(look, value, enabled, move |on| {
            act(move |state| set(state, on))
        }))
        .enabled(enabled)
}

pub fn choice<'a, T>(
    look: Look,
    value: T,
    enabled: bool,
    on_select: impl Fn(T) -> Message + 'a,
) -> Element<'a, Message>
where
    T: Choice + fmt::Debug + Send + Sync,
{
    let options: Vec<Labeled<T>> = T::ALL.iter().copied().map(Labeled).collect();
    let picker = pick_list(options, Some(Labeled(value)), move |Labeled(choice)| {
        on_select(choice)
    })
    .font(look.font)
    .text_size(look.sized(14.0))
    .style(style::choice(look))
    .menu_style(style::menu(look));
    if enabled {
        picker.into()
    } else {
        text(value.label())
            .font(look.font)
            .style(style::dimmed(look))
            .into()
    }
}

pub fn choice_row<'a, T>(
    look: Look,
    title: &'a str,
    value: T,
    set: fn(&AppState, T),
) -> PreferenceRow<'a>
where
    T: Choice + fmt::Debug + Send + Sync,
{
    choice_row_when(look, title, value, set, true)
}

pub fn choice_row_when<'a, T>(
    look: Look,
    title: &'a str,
    value: T,
    set: fn(&AppState, T),
    enabled: bool,
) -> PreferenceRow<'a>
where
    T: Choice + fmt::Debug + Send + Sync,
{
    preference_row(look, title)
        .suffix(choice(look, value, enabled, move |chosen| {
            act(move |state| set(state, chosen))
        }))
        .enabled(enabled)
}

pub struct SpinRange {
    pub values: RangeInclusive<u32>,
    pub step: u32,
}

pub fn spin_row<'a>(
    look: Look,
    title: &'a str,
    value: u32,
    range: SpinRange,
    set: fn(&AppState, u32),
    enabled: bool,
) -> PreferenceRow<'a> {
    let (lowest, highest) = (*range.values.start(), *range.values.end());
    let lower = value.saturating_sub(range.step).max(lowest);
    let higher = value.saturating_add(range.step).min(highest);
    let change_to = move |target: u32| {
        (enabled && target != value).then(|| act(move |state| set(state, target)))
    };

    let spinner = row![
        icon_button(look, Symbol::Remove, "Decrease", change_to(lower)),
        text(value.to_string())
            .font(look.mono)
            .size(look.sized(13.0))
            .width(44)
            .center(),
        icon_button(look, Symbol::Add, "Increase", change_to(higher)),
    ]
    .spacing(2)
    .align_y(Alignment::Center);

    preference_row(look, title).suffix(spinner).enabled(enabled)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NamedEntry {
    Name(String),
    NothingChosen,
    Action(&'static str),
}

impl fmt::Display for NamedEntry {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Name(name) => name,
            Self::NothingChosen => NOTHING_CHOSEN_LABEL,
            Self::Action(label) => label,
        })
    }
}

pub struct ChoiceAction {
    pub label: &'static str,
    pub message: Message,
}

pub fn named_choice_row<'a>(
    look: Look,
    title: &'a str,
    names: Vec<String>,
    chosen: String,
    set: fn(&AppState, String),
    action: Option<ChoiceAction>,
) -> PreferenceRow<'a> {
    let has_action = action.is_some();
    let mut entries: Vec<NamedEntry> = names.into_iter().map(NamedEntry::Name).collect();
    if has_action && entries.is_empty() {
        entries.push(NamedEntry::NothingChosen);
    }
    let action_message = action.map(|action| {
        entries.push(NamedEntry::Action(action.label));
        action.message
    });

    let selected = entries
        .iter()
        .find(|entry| matches!(entry, NamedEntry::Name(name) if *name == chosen))
        .or_else(|| {
            entries
                .iter()
                .find(|entry| **entry == NamedEntry::NothingChosen)
        })
        .cloned();

    let picker = pick_list(entries, selected, move |entry| match entry {
        NamedEntry::Name(name) => act(move |state| set(state, name.clone())),
        NamedEntry::Action(_) => action_message.clone().unwrap_or(Message::Ignored),
        NamedEntry::NothingChosen => Message::Ignored,
    })
    .font(look.font)
    .text_size(look.sized(14.0))
    .style(style::choice(look))
    .menu_style(style::menu(look));

    preference_row(look, title).suffix(picker)
}

pub fn keep_named_choice_valid(
    state: &AppState,
    options_property: &'static str,
    names: fn(&AppState) -> Vec<String>,
    chosen: fn(&AppState) -> String,
    set: fn(&AppState, String),
) {
    state.follow_properties(&[options_property], move |state| {
        let names = names(state);
        if let Some(first_name) = names.first()
            && !names.contains(&chosen(state))
        {
            set(state, first_name.clone());
        }
    });
}
