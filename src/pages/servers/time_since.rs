use std::time::{SystemTime, UNIX_EPOCH};

const MINUTE: i64 = 60;
const HOUR: i64 = 60 * MINUTE;
const DAY: i64 = 24 * HOUR;

pub fn time_since(timestamp: i64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|since_epoch| i64::try_from(since_epoch.as_secs()).ok())
        .unwrap_or(timestamp);
    let seconds = (now - timestamp).max(0);

    if seconds < HOUR {
        format!("{} min ago", (seconds / MINUTE).max(1))
    } else if seconds < DAY {
        format!("{} h ago", seconds / HOUR)
    } else {
        match seconds / DAY {
            1 => "1 day ago".to_owned(),
            days => format!("{days} days ago"),
        }
    }
}
