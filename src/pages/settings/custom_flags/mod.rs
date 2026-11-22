mod transfer;

use crate::components::icon::Symbol;
use crate::components::ignored_flag_icon::ignored_flag_icon;
use crate::components::preferences::{icon_button, preference_group, push_button};
use crate::components::style::{self, Emphasis, Look};
use crate::pages::settings::SettingsMessage;
use crate::pages::settings::fast_flag_allowlist::is_listed;
use crate::quadra::Message;
use crate::state::{AppState, FastFlagValue, FastFlags};
use iced::widget::{Id, operation, row, text_input};
use iced::{Alignment, Element, Task};
use std::collections::BTreeMap;

pub use transfer::{EXPORT_FILE_NAME, export_dialog, import_flags, save_exported};

struct FlagRow {
    id: Id,
    text: String,
}

#[derive(Debug, Clone)]
pub enum CustomFlagMessage {
    Add,
    Changed(usize, String),
    Remove(usize),
}

pub struct CustomFlags(Vec<FlagRow>);

impl CustomFlags {
    pub fn from_state(state: &AppState) -> Self {
        Self(
            state
                .fast_flags()
                .0
                .iter()
                .filter(|(name, _)| !is_listed(name))
                .map(|(name, value)| FlagRow {
                    id: Id::unique(),
                    text: format!("{name}={}", value.as_text()),
                })
                .collect(),
        )
    }

    pub fn update(&mut self, state: &AppState, message: CustomFlagMessage) -> Task<Message> {
        match message {
            CustomFlagMessage::Add => {
                let id = Id::unique();
                self.0.push(FlagRow {
                    id: id.clone(),
                    text: String::new(),
                });
                return Task::batch([
                    operation::focus(id),
                    operation::snap_to_end(crate::pages::settings::SCROLLER),
                ]);
            }
            CustomFlagMessage::Changed(position, text) => {
                if let Some(row) = self.0.get_mut(position) {
                    row.text = text;
                }
            }
            CustomFlagMessage::Remove(position) => {
                if position < self.0.len() {
                    self.0.remove(position);
                }
            }
        }
        self.save(state);
        Task::none()
    }

    fn save(&self, state: &AppState) {
        let mut flags: BTreeMap<String, FastFlagValue> = state
            .fast_flags()
            .0
            .into_iter()
            .filter(|(name, _)| is_listed(name))
            .collect();

        for row in &self.0 {
            let (name, value) = flag_of(&row.text);
            if !name.is_empty() && !is_listed(name) {
                flags.insert(name.to_owned(), FastFlagValue::parse(value));
            }
        }
        state.set_fast_flags(FastFlags(flags));
    }
}

fn message(message: CustomFlagMessage) -> Message {
    Message::Settings(SettingsMessage::CustomFlag(message))
}

pub fn custom_flags_group<'a>(flags: &'a CustomFlags, look: Look) -> Element<'a, Message> {
    let buttons = row![
        icon_button(
            look,
            Symbol::Add,
            "Add a flag",
            Some(message(CustomFlagMessage::Add))
        ),
        push_button(look, "Import", Emphasis::Flat).on_press(Message::ShowFlagImport),
        push_button(look, "Export", Emphasis::Flat).on_press(Message::ShowFlagExport),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    let rows = flags.0.iter().enumerate().map(|(position, flag)| {
        row![]
            .push(ignored_flag_icon(flag_of(&flag.text).0, look))
            .push(
                text_input("NAME=value", &flag.text)
                    .id(flag.id.clone())
                    .font(look.mono)
                    .size(look.sized(13.0))
                    .padding([7, 10])
                    .style(style::input(look, true))
                    .on_input(move |text| message(CustomFlagMessage::Changed(position, text))),
            )
            .push(icon_button(
                look,
                Symbol::Trash,
                "Remove this flag",
                Some(message(CustomFlagMessage::Remove(position))),
            ))
            .spacing(8)
            .padding([6, 12])
            .align_y(Alignment::Center)
    });

    preference_group(look)
        .title("Custom flags")
        .description(
            "Any other FastFlag, written as NAME=value. Roblox only honours allowlisted flags.",
        )
        .header_suffix(buttons)
        .extend(rows)
        .into()
}

fn flag_of(text: &str) -> (&str, &str) {
    let (name, value) = text.split_once('=').unwrap_or((text, ""));
    (name.trim(), value)
}
