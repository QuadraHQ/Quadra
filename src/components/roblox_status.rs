use crate::state::{AppState, Client};

pub const PROPERTIES_THAT_DECIDE_STATUS: [&str; 11] = [
    "busy",
    "running",
    "running-client",
    "selected-client",
    "roblox-graphics-started",
    "task-description",
    "task-progress",
    "roblox-installed",
    "roblox-version",
    "studio-installed",
    "studio-version",
];

const QUADRA_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn status_text(state: &AppState, client: Client) -> String {
    let is_opening = state.is_running(client) && !state.roblox_graphics_started();

    if state.busy() {
        let percent = (state.task_progress() * 100.0).round();
        if percent > 0.0 {
            format!("{} ({percent}%)", state.task_description())
        } else {
            format!("{}…", state.task_description())
        }
    } else if is_opening {
        format!("Opening {}…", client.name())
    } else if state.is_installed(client) {
        format!(
            "{} {} • Quadra {}",
            client.name(),
            state.version_of(client),
            QUADRA_VERSION
        )
    } else {
        format!("{} isn't installed", client.name())
    }
}
