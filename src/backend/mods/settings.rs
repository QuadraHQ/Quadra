use crate::state::{AppState, CursorStyle, DeathSound};
use std::path::PathBuf;

pub struct ModSettings {
    pub death_sound: DeathSound,
    pub custom_death_sound: PathBuf,
    pub old_character_sounds: bool,
    pub cursor_style: CursorStyle,
    pub custom_cursor: PathBuf,
    pub custom_font: PathBuf,
    pub custom_mods: bool,
}

impl ModSettings {
    pub fn of(state: &AppState) -> Self {
        Self {
            death_sound: state.death_sound(),
            custom_death_sound: state.custom_death_sound().into(),
            old_character_sounds: state.old_character_sounds(),
            cursor_style: state.cursor_style(),
            custom_cursor: state.custom_cursor().into(),
            custom_font: state.custom_font().into(),
            custom_mods: state.custom_mods(),
        }
    }
}
