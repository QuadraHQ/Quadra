use crate::backend;
use crate::state::{AppState, Client};

pub const PROPERTIES_THAT_DECIDE_PLAY_ACTION: [&str; 11] = [
    "running",
    "running-client",
    "selected-client",
    "stopping",
    "roblox-graphics-started",
    "busy",
    "can-play",
    "can-open-studio",
    "roblox-installed",
    "studio-installed",
    "rosemary-installed",
];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PlayAction {
    Install(Client),
    Play(Client),
    Opening,
    Stop,
    Stopping,
}

impl PlayAction {
    pub fn for_client(state: &AppState, client: Client) -> Self {
        let is_running = state.is_running(client);

        if is_running && state.stopping() {
            Self::Stopping
        } else if is_running && !state.roblox_graphics_started() {
            Self::Opening
        } else if is_running {
            Self::Stop
        } else if !state.is_installed(client) {
            Self::Install(client)
        } else {
            Self::Play(client)
        }
    }

    pub fn is_available(self, state: &AppState) -> bool {
        match self {
            Self::Install(_) => !state.busy(),
            Self::Play(client) => state.can_start(client) && !state.busy() && !state.running(),
            Self::Stop => true,
            Self::Opening | Self::Stopping => false,
        }
    }

    pub fn explanation_when_unavailable(self, state: &AppState) -> Option<&'static str> {
        match self {
            Self::Play(_) if state.running() => match state.running_client() {
                Client::Player => Some("Roblox is already running."),
                Client::Studio => Some("Roblox Studio is already running."),
            },
            Self::Play(client) if !state.can_start(client) => {
                Some("The selected runner isn't installed. Choose another one in Settings.")
            }
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Install(Client::Player) => "Install Roblox",
            Self::Install(Client::Studio) => "Install Roblox Studio",
            Self::Play(_) => "Play",
            Self::Opening => "Opening…",
            Self::Stop => "Stop",
            Self::Stopping => "Stopping…",
        }
    }

    pub fn icon_name(self) -> &'static str {
        match self {
            Self::Install(_) => "folder-download-symbolic",
            Self::Play(_) | Self::Opening => "media-playback-start-symbolic",
            Self::Stop | Self::Stopping => "media-playback-stop-symbolic",
        }
    }

    pub fn perform(self, state: &AppState) {
        match self {
            Self::Install(client) => backend::install(state, client),
            Self::Play(Client::Player) => backend::play(state),
            Self::Play(Client::Studio) => backend::open_studio(state),
            Self::Stop => backend::stop(state),
            Self::Opening | Self::Stopping => {}
        }
    }
}
