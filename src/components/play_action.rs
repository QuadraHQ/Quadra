use crate::backend;
use crate::state::AppState;

pub const PROPERTIES_THAT_DECIDE_PLAY_ACTION: [&str; 7] = [
    "running",
    "stopping",
    "roblox-graphics-started",
    "busy",
    "can-play",
    "roblox-installed",
    "rosemary-installed",
];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PlayAction {
    InstallRoblox,
    Play,
    Opening,
    Stop,
    Stopping,
}

impl PlayAction {
    pub fn for_state(state: &AppState) -> Self {
        if state.running() && state.stopping() {
            Self::Stopping
        } else if state.running() && !state.roblox_graphics_started() {
            Self::Opening
        } else if state.running() {
            Self::Stop
        } else if !state.roblox_installed() {
            Self::InstallRoblox
        } else {
            Self::Play
        }
    }

    pub fn is_available(self, state: &AppState) -> bool {
        match self {
            Self::InstallRoblox => !state.busy(),
            Self::Play => state.can_play() && !state.busy(),
            Self::Stop => true,
            Self::Opening | Self::Stopping => false,
        }
    }

    pub fn explanation_when_unavailable(self, state: &AppState) -> Option<&'static str> {
        match self {
            Self::Play if !state.can_play() => {
                Some("The selected runner isn't installed. Choose another one in Settings.")
            }
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::InstallRoblox => "Install Roblox",
            Self::Play => "Play",
            Self::Opening => "Opening…",
            Self::Stop => "Stop",
            Self::Stopping => "Stopping…",
        }
    }

    pub fn icon_name(self) -> &'static str {
        match self {
            Self::InstallRoblox => "folder-download-symbolic",
            Self::Play | Self::Opening => "media-playback-start-symbolic",
            Self::Stop | Self::Stopping => "media-playback-stop-symbolic",
        }
    }

    pub fn perform(self, state: &AppState) {
        match self {
            Self::InstallRoblox => backend::install_roblox(state),
            Self::Play => backend::play(state),
            Self::Stop => backend::stop(state),
            Self::Opening | Self::Stopping => {}
        }
    }
}
