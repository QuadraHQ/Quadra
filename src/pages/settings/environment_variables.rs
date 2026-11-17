use super::SettingsMessage;
use super::bound_rows::switch;
use crate::components::icon::Symbol;
use crate::components::preferences::{icon_button, preference_group, with_tooltip};
use crate::components::style::{self, Look};
use crate::quadra::Message;
use crate::state::{AppState, EnvironmentVariable, EnvironmentVariables};
use iced::widget::{Id, operation, row, text_input};
use iced::{Alignment, Element, Task};

struct VariableRow {
    id: Id,
    text: String,
    enabled: bool,
}

#[derive(Debug, Clone)]
pub enum VariableMessage {
    Add,
    Changed(usize, String),
    Toggled(usize, bool),
    Remove(usize),
}

pub struct EnvironmentVariableRows(Vec<VariableRow>);

impl EnvironmentVariableRows {
    pub fn from_state(state: &AppState) -> Self {
        Self(
            state
                .environment_variables()
                .0
                .iter()
                .map(|variable| VariableRow {
                    id: Id::unique(),
                    text: variable_as_text(variable),
                    enabled: variable.enabled,
                })
                .collect(),
        )
    }

    pub fn update(&mut self, state: &AppState, message: VariableMessage) -> Task<Message> {
        let mut focus = Task::none();
        match message {
            VariableMessage::Add => {
                let id = Id::unique();
                focus = operation::focus(id.clone());
                self.0.push(VariableRow {
                    id,
                    text: String::new(),
                    enabled: true,
                });
            }
            VariableMessage::Changed(position, text) => {
                if let Some(row) = self.0.get_mut(position) {
                    row.text = text;
                }
            }
            VariableMessage::Toggled(position, enabled) => {
                if let Some(row) = self.0.get_mut(position) {
                    row.enabled = enabled;
                }
            }
            VariableMessage::Remove(position) => {
                if position < self.0.len() {
                    self.0.remove(position);
                }
            }
        }
        self.save(state);
        focus
    }

    fn save(&self, state: &AppState) {
        let variables = self
            .0
            .iter()
            .map(|row| variable_from_text(&row.text, row.enabled))
            .collect();
        state.set_environment_variables(EnvironmentVariables(variables));
    }
}

fn message(message: VariableMessage) -> Message {
    Message::Settings(SettingsMessage::EnvironmentVariable(message))
}

pub fn environment_variables_group<'a>(
    rows: &'a EnvironmentVariableRows,
    look: Look,
) -> Element<'a, Message> {
    let variable_rows = rows.0.iter().enumerate().map(|(position, variable)| {
        row![
            text_input("NAME=value", &variable.text)
                .id(variable.id.clone())
                .font(look.mono)
                .size(13)
                .padding([7, 10])
                .style(style::input(look, true))
                .on_input(move |text| message(VariableMessage::Changed(position, text))),
            with_tooltip(
                look,
                switch(look, variable.enabled, true, move |enabled| {
                    message(VariableMessage::Toggled(position, enabled))
                }),
                "Pass this variable to Rosemary",
            ),
            icon_button(
                look,
                Symbol::Trash,
                "Remove this variable",
                Some(message(VariableMessage::Remove(position))),
            ),
        ]
        .spacing(8)
        .padding([6, 12])
        .align_y(Alignment::Center)
    });

    preference_group(look)
        .title("Environment variables")
        .description(
            "Extra variables passed to Rosemary when Roblox starts, written as NAME=value.",
        )
        .header_suffix(icon_button(
            look,
            Symbol::Add,
            "Add a variable",
            Some(message(VariableMessage::Add)),
        ))
        .extend(variable_rows)
        .into()
}

fn variable_as_text(variable: &EnvironmentVariable) -> String {
    if variable.name.is_empty() && variable.value.is_empty() {
        String::new()
    } else {
        format!("{}={}", variable.name, variable.value)
    }
}

fn variable_from_text(text: &str, enabled: bool) -> EnvironmentVariable {
    let (name, value) = text.split_once('=').unwrap_or((text, ""));
    EnvironmentVariable {
        name: name.trim().to_owned(),
        value: value.to_owned(),
        enabled,
    }
}
