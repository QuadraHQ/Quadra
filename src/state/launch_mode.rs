use super::Choice;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
