use crate::state::Client;
use std::path::PathBuf;

pub enum StudioTarget {
    StartPage,
    Link(String),
    PlaceFile(PathBuf),
}

pub enum LaunchRequest {
    Roblox { link: Option<String> },
    Studio(StudioTarget),
}

impl LaunchRequest {
    pub fn client(&self) -> Client {
        match self {
            Self::Roblox { .. } => Client::Player,
            Self::Studio(_) => Client::Studio,
        }
    }
}
