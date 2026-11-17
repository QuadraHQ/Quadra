use super::bound_rows::choice;
use super::custom_flags::custom_flags_group;
use super::fast_flag_allowlist::{
    ALLOWLIST, ALLOWLIST_SOURCE, AllowlistedFlag, FlagCategory, FlagKind,
};
use super::{Settings, SettingsMessage};
use crate::components::preferences::{
    PreferenceRow, preference_group, preference_row, push_button, with_tooltip,
};
use crate::components::style::{self, Emphasis, Look};
use crate::quadra::{Message, act};
use crate::state::{AppState, Choice, FastFlagValue, FastFlags};
use iced::Element;
use iced::widget::text_input;
use std::collections::HashMap;

const NUMBER_INPUT_WIDTH: f32 = 110.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToggleSetting {
    Default,
    On,
    Off,
}

impl Choice for ToggleSetting {
    const ALL: &'static [Self] = &[Self::Default, Self::On, Self::Off];

    fn label(self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::On => "On",
            Self::Off => "Off",
        }
    }
}

impl ToggleSetting {
    fn of(value: Option<FastFlagValue>) -> Self {
        match value {
            Some(FastFlagValue::Toggle(true)) => Self::On,
            Some(FastFlagValue::Toggle(false)) => Self::Off,
            _ => Self::Default,
        }
    }

    fn value(self) -> Option<FastFlagValue> {
        match self {
            Self::Default => None,
            Self::On => Some(FastFlagValue::Toggle(true)),
            Self::Off => Some(FastFlagValue::Toggle(false)),
        }
    }
}

pub fn fast_flags_section<'a>(
    settings: &'a Settings,
    state: &AppState,
    look: Look,
) -> Vec<Element<'a, Message>> {
    let about_group = preference_group(look)
        .title("FastFlags")
        .description(
            "Roblox only applies flags on its allowlist. Quadra writes the flags you set to ClientAppSettings.json each time Roblox starts.",
        )
        .header_suffix(
            push_button(look, "View the allowlist", Emphasis::Flat)
                .on_press(Message::Open(ALLOWLIST_SOURCE.to_owned())),
        );

    let flags = state.fast_flags();
    let mut sections = vec![about_group.into()];
    sections.extend(
        ALLOWLIST
            .iter()
            .map(|category| category_group(settings, &flags, category, look)),
    );
    sections.push(custom_flags_group(&settings.custom_flags, look));
    sections
}

fn category_group<'a>(
    settings: &'a Settings,
    flags: &FastFlags,
    category: &'static FlagCategory,
    look: Look,
) -> Element<'a, Message> {
    let rows = category.flags.iter().map(|flag| match flag.kind {
        FlagKind::Toggle => toggle_row(flags.get(flag.name), flag, look),
        FlagKind::Number => number_row(settings, flags.get(flag.name), flag, look),
    });
    preference_group(look)
        .title(category.title)
        .extend(rows)
        .into()
}

fn toggle_row<'a>(
    value: Option<FastFlagValue>,
    flag: &'static AllowlistedFlag,
    look: Look,
) -> PreferenceRow<'a> {
    preference_row(look, flag.title)
        .monospace_subtitle(flag.name)
        .suffix(choice(
            look,
            ToggleSetting::of(value),
            true,
            move |setting| act(move |state| set_flag(state, flag.name, setting.value())),
        ))
}

fn number_row<'a>(
    settings: &'a Settings,
    value: Option<FastFlagValue>,
    flag: &'static AllowlistedFlag,
    look: Look,
) -> PreferenceRow<'a> {
    let invalid = settings.invalid_numbers.get(flag.name);
    let shown = match (invalid, value) {
        (Some(typed), _) => typed.clone(),
        (None, Some(FastFlagValue::Number(number))) => number.to_string(),
        (None, _) => String::new(),
    };

    let input = text_input("Default", &shown)
        .font(look.mono)
        .size(13)
        .width(NUMBER_INPUT_WIDTH)
        .align_x(iced::Alignment::End)
        .padding([6, 10])
        .style(style::input(look, invalid.is_none()))
        .on_input(move |text| {
            Message::Settings(SettingsMessage::NumberFlagChanged(flag.name, text))
        });
    let input = with_tooltip(
        look,
        input,
        "Enter a whole number, or leave it empty for the default",
    );

    preference_row(look, flag.title)
        .monospace_subtitle(flag.name)
        .suffix(input)
}

pub fn number_changed(
    state: &AppState,
    invalid_numbers: &mut HashMap<&'static str, String>,
    name: &'static str,
    text: String,
) {
    let typed = text.trim();
    if typed.is_empty() {
        invalid_numbers.remove(name);
        set_flag(state, name, None);
    } else if let Ok(number) = typed.parse() {
        invalid_numbers.remove(name);
        set_flag(state, name, Some(FastFlagValue::Number(number)));
    } else {
        invalid_numbers.insert(name, text);
    }
}

fn set_flag(state: &AppState, name: &str, value: Option<FastFlagValue>) {
    state.set_fast_flags(state.fast_flags().with(name, value));
}
