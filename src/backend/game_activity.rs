const JOINING_GAME: &str = "! Joining game '";
const PLACE_AFTER_JOB_ID: &str = "' place ";
const WORD_BEFORE_SERVER_ADDRESS: &str = "at";
const CONNECTION_ACCEPTED: &str = "Connection accepted from ";
const JOIN_REPORT: &str = "[FLog::GameJoinLoadTime] Report game_join_loadtime:";
const UNIVERSE_ID_FIELD: &str = "universeid:";
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
    Connected {
        server_ip: String,
    },
    JoinReported {
        universe_id: u64,
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

    if let Some((_, address)) = line.split_once(CONNECTION_ACCEPTED) {
        let server_ip = address.split(['|', ' ', ':']).next()?;
        return Some(GameEvent::Connected {
            server_ip: server_ip.to_owned(),
        });
    }

    if let Some((_, report)) = line.split_once(JOIN_REPORT) {
        let (_, universe_id_onwards) = report.split_once(UNIVERSE_ID_FIELD)?;
        let universe_id: String = universe_id_onwards
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        return Some(GameEvent::JoinReported {
            universe_id: universe_id.parse().ok()?,
        });
    }

    LEFT_GAME_MARKERS
        .iter()
        .any(|marker| line.contains(marker))
        .then_some(GameEvent::Left)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_public_server_ip_from_the_connection_line() {
        let line = "2026-10-06T00:06:41.779Z,10.779502,9d4bb000,6,Info [DFLog::NetworkClient] Connection accepted from 128.116.48.33|51019";
        assert_eq!(
            events_in(line).collect::<Vec<_>>(),
            [GameEvent::Connected {
                server_ip: "128.116.48.33".to_owned()
            }]
        );
    }

    #[test]
    fn reads_the_join_line() {
        let line = "[FLog::Output] ! Joining game 'abc-1' place 123 at 10.17.3.215";
        assert_eq!(
            events_in(line).collect::<Vec<_>>(),
            [GameEvent::Joining {
                place_id: 123,
                job_id: "abc-1".to_owned(),
                server_address: Some("10.17.3.215".to_owned()),
            }]
        );
    }

    #[test]
    fn reads_the_universe_id_from_the_join_report() {
        let line = "[FLog::GameJoinLoadTime] Report game_join_loadtime: placeid:1818, visitid:0, universeid:13058, referral_page:Unknown, userid:42";
        assert_eq!(
            events_in(line).collect::<Vec<_>>(),
            [GameEvent::JoinReported { universe_id: 13058 }]
        );
    }
}
