use crate::state::AppState;

pub const PROPERTIES_THAT_DECIDE_STATUS: [&str; 5] = [
    "busy",
    "task-description",
    "task-progress",
    "roblox-installed",
    "roblox-version",
];

pub fn status_text(state: &AppState) -> String {
    if state.busy() {
        let percent = (state.task_progress() * 100.0).round();
        if percent > 0.0 {
            format!("{} ({percent}%)", state.task_description())
        } else {
            format!("{}…", state.task_description())
        }
    } else if state.roblox_installed() {
        format!("Roblox {}", state.roblox_version())
    } else {
        "Roblox isn't installed".to_owned()
    }
}
