use gtk::glib;
use serde::{Deserialize, Serialize};

const MAX_REMEMBERED_GAMES: usize = 50;
const MAX_REMEMBERED_SERVERS_PER_GAME: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JoinedServer {
    pub job_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    pub joined_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayedGame {
    pub place_id: u64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    pub servers: Vec<JoinedServer>,
}

impl PlayedGame {
    pub fn last_played_at(&self) -> Option<i64> {
        self.servers.first().map(|server| server.joined_at)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, glib::Boxed, Serialize, Deserialize)]
#[boxed_type(name = "QuadraServerHistory")]
#[serde(transparent)]
pub struct ServerHistory(pub Vec<PlayedGame>);

impl ServerHistory {
    pub fn with_join(&self, place_id: u64, mut server: JoinedServer) -> Self {
        let mut games = self.0.clone();
        let mut game = match games.iter().position(|game| game.place_id == place_id) {
            Some(position) => games.remove(position),
            None => PlayedGame {
                place_id,
                name: String::new(),
                servers: Vec::new(),
            },
        };

        if let Some(position) = game
            .servers
            .iter()
            .position(|joined| joined.job_id == server.job_id)
        {
            let earlier_join = game.servers.remove(position);
            server.location = server.location.or(earlier_join.location);
            server.address = server.address.or(earlier_join.address);
        }
        game.servers.insert(0, server);
        game.servers.truncate(MAX_REMEMBERED_SERVERS_PER_GAME);

        games.insert(0, game);
        games.truncate(MAX_REMEMBERED_GAMES);
        Self(games)
    }

    pub fn with_game_name(&self, place_id: u64, name: &str) -> Self {
        let mut games = self.0.clone();
        for game in games.iter_mut().filter(|game| game.place_id == place_id) {
            game.name = name.to_owned();
        }
        Self(games)
    }

    pub fn has_name_for(&self, place_id: u64) -> bool {
        self.0
            .iter()
            .any(|game| game.place_id == place_id && !game.name.is_empty())
    }

    pub fn job_ids_in(&self, place_id: u64) -> Vec<String> {
        self.0
            .iter()
            .filter(|game| game.place_id == place_id)
            .flat_map(|game| game.servers.iter().map(|server| server.job_id.clone()))
            .collect()
    }
}
