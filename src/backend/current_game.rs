use super::game_activity::{self, GameEvent};
use crate::state::AppState;
use crate::utils::seconds_since_epoch;

pub fn initialize(state: &AppState) {
    state.connect_log_appended(|state, output| {
        for event in game_activity::events_in(output) {
            match event {
                GameEvent::Joining { .. } => state.set_in_game(true),
                GameEvent::Left => state.set_in_game(false),
                GameEvent::Connected { .. } => {}
            }
        }
    });

    state.connect_running_notify(|state| {
        if state.running() {
            state.set_session_started_at(seconds_since_epoch());
        } else {
            state.set_in_game(false);
        }
    });
}
