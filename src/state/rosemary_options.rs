use super::Choice;
use gtk::glib;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, glib::Enum, Serialize, Deserialize)]
#[enum_type(name = "QuadraPresentationMode")]
#[serde(rename_all = "snake_case")]
pub enum PresentationMode {
    #[default]
    Default,
    Immediate,
    Fifo,
    FifoRelaxed,
    Mailbox,
}

impl Choice for PresentationMode {
    const ALL: &'static [Self] = &[
        Self::Default,
        Self::Immediate,
        Self::Fifo,
        Self::FifoRelaxed,
        Self::Mailbox,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::Immediate => "Immediate (no V-Sync)",
            Self::Fifo => "FIFO (V-Sync)",
            Self::FifoRelaxed => "FIFO relaxed (adaptive V-Sync)",
            Self::Mailbox => "Mailbox",
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, glib::Enum, Serialize, Deserialize)]
#[enum_type(name = "QuadraSwapInterval")]
#[serde(rename_all = "snake_case")]
pub enum SwapInterval {
    #[default]
    Default,
    Immediate,
    VSync,
    AdaptiveVSync,
}

impl Choice for SwapInterval {
    const ALL: &'static [Self] = &[
        Self::Default,
        Self::Immediate,
        Self::VSync,
        Self::AdaptiveVSync,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::Immediate => "Immediate (no V-Sync)",
            Self::VSync => "V-Sync",
            Self::AdaptiveVSync => "Adaptive V-Sync",
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, glib::Enum, Serialize, Deserialize)]
#[enum_type(name = "QuadraTitleBar")]
#[serde(rename_all = "snake_case")]
pub enum TitleBar {
    #[default]
    MacStyle,
    System,
}

impl Choice for TitleBar {
    const ALL: &'static [Self] = &[Self::MacStyle, Self::System];

    fn label(self) -> &'static str {
        match self {
            Self::MacStyle => "Mac-style",
            Self::System => "System",
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, glib::Enum, Serialize, Deserialize)]
#[enum_type(name = "QuadraCommandKey")]
#[serde(rename_all = "snake_case")]
pub enum CommandKey {
    #[default]
    Super,
    Ctrl,
    Alt,
}

impl Choice for CommandKey {
    const ALL: &'static [Self] = &[Self::Super, Self::Ctrl, Self::Alt];

    fn label(self) -> &'static str {
        match self {
            Self::Super => "Super",
            Self::Ctrl => "Ctrl",
            Self::Alt => "Alt",
        }
    }
}
