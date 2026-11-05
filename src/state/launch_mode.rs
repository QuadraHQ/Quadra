use super::Choice;
use gtk::glib;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, glib::Enum, Serialize, Deserialize)]
#[enum_type(name = "QuadraLaunchMode")]
#[serde(rename_all = "snake_case")]
pub enum LaunchMode {
    #[default]
    OpenQuadra,
    LaunchRoblox,
}

impl Choice for LaunchMode {
    const ALL: &'static [Self] = &[Self::OpenQuadra, Self::LaunchRoblox];

    fn label(self) -> &'static str {
        match self {
            Self::OpenQuadra => "Open Quadra",
            Self::LaunchRoblox => "Launch Roblox",
        }
    }
}
