const JOINING_GAME: &str = "! Joining game '";
const PLACE_AFTER_JOB_ID: &str = "' place ";
const LEFT_GAME_MARKERS: [&str; 2] = [
    "[FLog::Network] Time to disconnect replication data:",
    "[FLog::SingleSurfaceApp] leaveUGCGameInternal",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameEvent {
    Joining { place_id: u64 },
    Left,
}

pub fn events_in(output: &str) -> impl Iterator<Item = GameEvent> + '_ {
    output.lines().filter_map(event_in_line)
}

fn event_in_line(line: &str) -> Option<GameEvent> {
    if let Some((_, job_and_place)) = line.split_once(JOINING_GAME) {
        let (_, place_and_server) = job_and_place.split_once(PLACE_AFTER_JOB_ID)?;
        let place_id = place_and_server.split_whitespace().next()?.parse().ok()?;
        return Some(GameEvent::Joining { place_id });
    }

    LEFT_GAME_MARKERS
        .iter()
        .any(|marker| line.contains(marker))
        .then_some(GameEvent::Left)
}
