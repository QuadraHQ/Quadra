use super::Choice;
use gtk::glib;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, glib::Enum, Serialize, Deserialize)]
#[enum_type(name = "QuadraDeathSound")]
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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, glib::Enum, Serialize, Deserialize)]
#[enum_type(name = "QuadraCursorStyle")]
#[serde(rename_all = "snake_case")]
pub enum CursorStyle {
    #[default]
    Stock,
    Classic2006,
    Classic2013,
    PurpleCross,
    Dot,
    Custom,
}

impl Choice for CursorStyle {
    const ALL: &'static [Self] = &[
        Self::Stock,
        Self::Classic2006,
        Self::Classic2013,
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
            Self::Custom => "Custom file or folder",
        }
    }
}
