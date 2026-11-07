use crate::state::AppState;

pub const PROPERTIES_THAT_DECIDE_STATUS: [&str; 7] = [
    "busy",
    "running",
    "roblox-graphics-started",
    "task-description",
    "task-progress",
    "roblox-installed",
    "roblox-version",
];

const QUADRA_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn status_text(state: &AppState) -> String {
    if state.busy() {
        let percent = (state.task_progress() * 100.0).round();
        if percent > 0.0 {
            format!("{} ({percent}%)", state.task_description())
        } else {
            format!("{}…", state.task_description())
        }
    } else if state.running() && !state.roblox_graphics_started() {
        "Opening Roblox…".to_owned()
    } else if state.roblox_installed() {
        format!(
            "Roblox {} • Quadra {}",
            state.roblox_version(),
            QUADRA_VERSION
        )
    } else {
        "Roblox isn't installed".to_owned()
    }
}
