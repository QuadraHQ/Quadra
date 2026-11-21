use super::Choice;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeathSound {
    #[default]
    Stock,
    ClassicOof,
    Custom,
}

impl Choice for DeathSound {
    const ALL: &'static [Self] = &[Self::Stock, Self::ClassicOof, Self::Custom];

    fn label(self) -> &'static str {
        match self {
            Self::Stock => "Default",
            Self::ClassicOof => "Classic OOF",
            Self::Custom => "Custom file",
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CursorStyle {
    #[default]
    Stock,
    Classic2006,
    Classic2013,
    PurpleCross,
    Dot,
    Modern,
    Dark,
    Neon,
    Crosshair,
    Ring,
    Custom,
}

impl Choice for CursorStyle {
    const ALL: &'static [Self] = &[
        Self::Stock,
        Self::Classic2006,
        Self::Classic2013,
        Self::Modern,
        Self::Dark,
        Self::Neon,
        Self::Crosshair,
        Self::Ring,
        Self::PurpleCross,
        Self::Dot,
        Self::Custom,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Stock => "Default",
            Self::Classic2006 => "2006",
            Self::Classic2013 => "2013",
            Self::PurpleCross => "Purple cross",
            Self::Dot => "Dot",
            Self::Modern => "Modern arrow",
            Self::Dark => "Dark arrow",
            Self::Neon => "Neon arrow",
            Self::Crosshair => "Crosshair",
            Self::Ring => "Ring",
            Self::Custom => "Custom file or folder",
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FontStyle {
    #[default]
    Stock,
    ComicNeue,
    PressStart,
    Bangers,
    Vt323,
    PatrickHand,
    AtkinsonHyperlegible,
    Custom,
}

impl Choice for FontStyle {
    const ALL: &'static [Self] = &[
        Self::Stock,
        Self::ComicNeue,
        Self::PressStart,
        Self::Bangers,
        Self::Vt323,
        Self::PatrickHand,
        Self::AtkinsonHyperlegible,
        Self::Custom,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Stock => "Default",
            Self::ComicNeue => "Comic Neue",
            Self::PressStart => "Press Start 2P",
            Self::Bangers => "Bangers",
            Self::Vt323 => "VT323",
            Self::PatrickHand => "Patrick Hand",
            Self::AtkinsonHyperlegible => "Atkinson Hyperlegible",
            Self::Custom => "Custom file",
        }
    }
}
