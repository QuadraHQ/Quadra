use super::fast_flag_allowlist::{
    ALLOWLIST, ALLOWLIST_SOURCE, AllowlistedFlag, FlagCategory, FlagKind,
};
use crate::state::{AppState, Choice, FastFlagValue};
use adw::prelude::*;
use gtk::glib;

const INVALID_NUMBER_STYLE: &str = "error";

#[derive(Clone, Copy, PartialEq, Eq)]
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

pub fn fast_flags_section(state: &AppState) -> gtk::Box {
    let allowlist_link = gtk::LinkButton::builder()
        .uri(ALLOWLIST_SOURCE)
        .label("View the allowlist")
        .valign(gtk::Align::Center)
        .build();

    let about_group = adw::PreferencesGroup::builder()
        .title("FastFlags")
        .description(
            "Roblox only applies flags on its allowlist. Quadra writes the flags you set to ClientAppSettings.json each time Roblox starts.",
        )
        .header_suffix(&allowlist_link)
        .build();

    let section = gtk::Box::new(gtk::Orientation::Vertical, 12);
    section.append(&about_group);
    for category in ALLOWLIST {
        section.append(&category_group(state, category));
    }

    section
}

fn category_group(state: &AppState, category: &FlagCategory) -> adw::PreferencesGroup {
    let group = adw::PreferencesGroup::builder()
        .title(category.title)
        .build();

    for flag in category.flags {
        match flag.kind {
            FlagKind::Toggle => group.add(&toggle_row(state, flag)),
            FlagKind::Number => group.add(&number_row(state, flag)),
        }
    }

    group
}

fn toggle_row(state: &AppState, flag: &'static AllowlistedFlag) -> adw::ComboRow {
    let current_setting = ToggleSetting::of(state.fast_flags().get(flag.name));

    let row = adw::ComboRow::builder()
        .title(flag.title)
        .subtitle(flag.name)
        .model(&gtk::StringList::new(&ToggleSetting::labels()))
        .selected(current_setting.position())
        .build();

    row.connect_selected_notify(glib::clone!(
        #[strong]
        state,
        move |row| {
            let setting =
                ToggleSetting::at_position(row.selected()).unwrap_or(ToggleSetting::Default);
            set_flag(&state, flag.name, setting.value());
        }
    ));

    row
}

fn number_row(state: &AppState, flag: &'static AllowlistedFlag) -> adw::ActionRow {
    let current_number = match state.fast_flags().get(flag.name) {
        Some(FastFlagValue::Number(number)) => number.to_string(),
        _ => String::new(),
    };

    let entry = gtk::Entry::builder()
        .text(current_number)
        .placeholder_text("Default")
        .input_purpose(gtk::InputPurpose::Number)
        .width_chars(8)
        .xalign(1.0)
        .valign(gtk::Align::Center)
        .build();

    let row = adw::ActionRow::builder()
        .title(flag.title)
        .subtitle(flag.name)
        .activatable_widget(&entry)
        .build();
    row.add_suffix(&entry);

    entry.connect_changed(glib::clone!(
        #[strong]
        state,
        move |entry| {
            let text = entry.text();
            let text = text.trim();
            if text.is_empty() {
                show_number_is_valid(entry, true);
                set_flag(&state, flag.name, None);
            } else if let Ok(number) = text.parse() {
                show_number_is_valid(entry, true);
                set_flag(&state, flag.name, Some(FastFlagValue::Number(number)));
            } else {
                show_number_is_valid(entry, false);
            }
        }
    ));

    row
}

fn show_number_is_valid(entry: &gtk::Entry, is_valid: bool) {
    if is_valid {
        entry.remove_css_class(INVALID_NUMBER_STYLE);
        entry.set_tooltip_text(None);
    } else {
        entry.add_css_class(INVALID_NUMBER_STYLE);
        entry.set_tooltip_text(Some(
            "Enter a whole number, or leave it empty for the default",
        ));
    }
}

fn set_flag(state: &AppState, name: &str, value: Option<FastFlagValue>) {
    state.set_fast_flags(state.fast_flags().with(name, value));
}
