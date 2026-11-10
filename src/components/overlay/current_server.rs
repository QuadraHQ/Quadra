use crate::state::{AppState, JoinedServer, PlayedGame};
use crate::utils::{format_clock, seconds_since_epoch};

pub struct CurrentServer {
    pub game: PlayedGame,
    pub server: JoinedServer,
}

impl CurrentServer {
    pub fn of(state: &AppState) -> Option<Self> {
        if !state.in_game() {
            return None;
        }
        let game = state.server_history().0.into_iter().next()?;
        let server = game.servers.first()?.clone();
        Some(Self { game, server })
    }

    pub fn game_name(&self) -> String {
        if self.game.name.is_empty() {
            format!("Place {}", self.game.place_id)
        } else {
            self.game.name.clone()
        }
    }

    pub fn time_on_server(&self) -> String {
        format_clock(seconds_since_epoch() - self.server.joined_at)
    }
}

pub fn session_time(state: &AppState) -> String {
    let started_at = match state.session_started_at() {
        0 => seconds_since_epoch(),
        started_at => started_at,
    };
    format_clock(seconds_since_epoch() - started_at)
}
