use super::icon::Symbol;
use super::preferences::{icon_button, preference_row};
use super::style::Look;
use crate::quadra::{Message, act};
use crate::state::AppState;
use crate::utils::path_for_display;
use iced::Element;
use std::path::Path;

const NOTHING_CHOSEN: &str = "Nothing chosen";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pick {
    File(&'static [&'static str]),
    Folder,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChosenPath {
    DeathSound,
    Cursor,
    Font,
}

impl ChosenPath {
    pub fn get(self, state: &AppState) -> String {
        match self {
            Self::DeathSound => state.custom_death_sound(),
            Self::Cursor => state.custom_cursor(),
            Self::Font => state.custom_font(),
        }
    }

    pub fn set(self, state: &AppState, path: impl Into<String>) {
        match self {
            Self::DeathSound => state.set_custom_death_sound(path),
            Self::Cursor => state.set_custom_cursor(path),
            Self::Font => state.set_custom_font(path),
        }
    }
}

pub fn file_row<'a>(
    state: &AppState,
    look: Look,
    chosen_path: ChosenPath,
    title: &'a str,
    picks: &'static [Pick],
) -> Element<'a, Message> {
    let path = chosen_path.get(state);
    let mut row = preference_row(look, title).subtitle(shown(&path));

    if !path.is_empty() {
        row = row.suffix(icon_button(
            look,
            Symbol::Clear,
            "Clear",
            Some(act(move |state| chosen_path.set(state, ""))),
        ));
    }
    for &pick in picks {
        let (symbol, explanation) = match pick {
            Pick::File(_) => (Symbol::File, "Choose a file"),
            Pick::Folder => (Symbol::Folder, "Choose a folder"),
        };
        row = row.suffix(icon_button(
            look,
            symbol,
            explanation,
            Some(Message::PickPath(chosen_path, pick)),
        ));
    }

    row.into()
}

fn shown(path: &str) -> String {
    if path.is_empty() {
        NOTHING_CHOSEN.to_owned()
    } else {
        path_for_display(Path::new(path))
    }
}
