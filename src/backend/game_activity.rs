const JOINING_GAME: &str = "! Joining game '";
const PLACE_AFTER_JOB_ID: &str = "' place ";
const WORD_BEFORE_SERVER_ADDRESS: &str = "at";
const LEFT_GAME_MARKERS: [&str; 2] = [
    "[FLog::Network] Time to disconnect replication data:",
    "[FLog::SingleSurfaceApp] leaveUGCGameInternal",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameEvent {
    Joining {
        place_id: u64,
        job_id: String,
        server_address: Option<String>,
    },
    Left,
}

pub fn events_in(output: &str) -> impl Iterator<Item = GameEvent> + '_ {
    output.lines().filter_map(event_in_line)
}

fn event_in_line(line: &str) -> Option<GameEvent> {
    if let Some((_, job_and_place)) = line.split_once(JOINING_GAME) {
        let (job_id, place_and_server) = job_and_place.split_once(PLACE_AFTER_JOB_ID)?;
        let mut words = place_and_server.split_whitespace();
        let place_id = words.next()?.parse().ok()?;
        let server_address = match (words.next(), words.next()) {
            (Some(WORD_BEFORE_SERVER_ADDRESS), Some(address)) => Some(address.to_owned()),
            _ => None,
        };
        return Some(GameEvent::Joining {
            place_id,
            job_id: job_id.to_owned(),
            server_address,
        });
    }

    LEFT_GAME_MARKERS
        .iter()
        .any(|marker| line.contains(marker))
        .then_some(GameEvent::Left)
}
