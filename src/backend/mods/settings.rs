use crate::components::imported_theme::brightness;
use crate::components::style::Look;
use crate::state::{AppState, CursorStyle, DeathSound};
use iced::font::Family;
use std::path::PathBuf;

pub struct RobloxTheme {
    pub accent: [f32; 4],
    pub outline: [f32; 4],
    pub font: Option<String>,
}

impl RobloxTheme {
    fn of(look: &Look) -> Self {
        let outline = if brightness(look.accent) > 0.35 {
            [0.04, 0.04, 0.06, 0.9]
        } else {
            [1.0, 1.0, 1.0, 0.9]
        };
        Self {
            accent: [look.accent.r, look.accent.g, look.accent.b, 1.0],
            outline,
            font: match look.font.family {
                Family::Name(name) => Some(name.to_owned()),
                _ => None,
            },
        }
    }
}

pub struct ModSettings {
    pub death_sound: DeathSound,
    pub custom_death_sound: PathBuf,
    pub old_character_sounds: bool,
    pub cursor_style: CursorStyle,
    pub custom_cursor: PathBuf,
    pub custom_font: PathBuf,
    pub custom_mods: bool,
    pub theme: Option<RobloxTheme>,
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
            theme: state
                .theme_roblox()
                .then(|| RobloxTheme::of(&Look::of(state))),
        }
    }
}
