use crate::backend;
use crate::components::preferences::{PreferenceRow, preference_row, push_button};
use crate::components::style::{Emphasis, Look};
use crate::quadra::act;
use crate::state::AppState;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn launcher_version_row<'a>(state: &AppState, look: Look) -> PreferenceRow<'a> {
    let status = state.update_status();
    let subtitle = if status.is_empty() {
        format!("v{VERSION}")
    } else {
        format!("v{VERSION}\n{status}")
    };
    let mut row = preference_row(look, "Launcher version").subtitle(subtitle);

    if backend::can_update_itself() {
        let is_idle = !state.busy();
        row = row.suffix(
            push_button(look, "Check for updates", Emphasis::Normal)
                .on_press_maybe(is_idle.then(|| act(backend::check_for_updates))),
        );
        if !state.available_update().is_empty() {
            row = row.suffix(
                push_button(look, "Update", Emphasis::Suggested)
                    .on_press_maybe(is_idle.then(|| act(backend::install_update))),
            );
        }
    }

    row
}
