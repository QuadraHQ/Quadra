use super::background::run_quietly;
use super::game_activity::{self, GameEvent};
use super::req_agent::api_agent;
use super::server_history;
use crate::state::AppState;
use anyhow::Context;
use notify_rust::{Hint, Notification};
use serde_json::Value;
use std::cell::RefCell;
use std::net::Ipv4Addr;
use std::sync::atomic::{AtomicU32, Ordering};

const GEOLOCATION_URL: &str = "https://apis.rovalra.com/v1/geolocation";
const NOTIFICATION_SUMMARY: &str = "Server joined";
const MAX_PART_LENGTH: usize = 64;

static SHOWN_NOTIFICATION: AtomicU32 = AtomicU32::new(0);

thread_local! {
    static CURRENT_JOB_ID: RefCell<Option<String>> = const { RefCell::new(None) };
}

pub fn initialize(state: &AppState) {
    state.connect_log_appended(|state, output| {
        for event in game_activity::events_in(output) {
            match event {
                GameEvent::Joining { job_id, .. } => {
                    CURRENT_JOB_ID.replace(Some(job_id));
                }
                GameEvent::Connected { server_ip } if state.notify_server_location() => {
                    look_up(state, &server_ip);
                }
                GameEvent::Connected { .. }
                | GameEvent::JoinReported { .. }
                | GameEvent::Left
                | GameEvent::PresenceUpdated(_) => {}
            }
        }
    });
}

fn look_up(state: &AppState, server_ip: &str) {
    let Some(address) = server_ip.parse().ok().filter(is_public) else {
        return;
    };
    let job_id = CURRENT_JOB_ID.with_borrow(Clone::clone);

    let state = state.clone();
    run_quietly(
        move || fetch_location(address),
        move |location| match location {
            Ok(location) => {
                notify(&location);
                if let Some(job_id) = job_id {
                    server_history::remember_location(&state, &job_id, &location);
                }
            }
            Err(error) => eprintln!("Couldn't find where the server is: {error:#}"),
        },
    );
}

fn fetch_location(address: Ipv4Addr) -> anyhow::Result<String> {
    let response: Value = api_agent()
        .get(GEOLOCATION_URL)
        .query("ip", address.to_string())
        .call()
        .and_then(|mut response| response.body_mut().read_json())
        .context("The location lookup failed")?;

    let parts: Vec<String> = ["city", "region", "country"]
        .iter()
        .filter_map(|part| response["location"][part].as_str())
        .filter(|part| !part.is_empty())
        .map(|part| part.chars().take(MAX_PART_LENGTH).collect())
        .collect();
    if parts.is_empty() {
        anyhow::bail!("RoValra doesn't know where {address} is");
    }
    Ok(parts.join(", "))
}

fn notify(location: &str) {
    let body = format!("Your server is located in {location}");
    std::thread::spawn(move || {
        let mut notification = Notification::new();
        notification
            .appname("Quadra")
            .summary(NOTIFICATION_SUMMARY)
            .body(&body)
            .icon(crate::APP_ID)
            .hint(Hint::DesktopEntry(crate::APP_ID.to_owned()));
        let previous = SHOWN_NOTIFICATION.load(Ordering::Relaxed);
        if previous != 0 {
            notification.id(previous);
        }
        match notification.show() {
            Ok(shown) => SHOWN_NOTIFICATION.store(shown.id(), Ordering::Relaxed),
            Err(error) => eprintln!("Couldn't show the server location: {error}"),
        }
    });
}

fn is_public(address: &Ipv4Addr) -> bool {
    let is_shared = address.octets()[0] == 100 && (64..128).contains(&address.octets()[1]);
    !(address.is_private()
        || address.is_loopback()
        || address.is_link_local()
        || address.is_unspecified()
        || address.is_broadcast()
        || address.is_multicast()
        || address.is_documentation()
        || is_shared)
}

#[cfg(test)]
mod tests {
    use super::is_public;

    #[test]
    fn only_public_addresses_are_looked_up() {
        assert!(is_public(&"128.116.48.33".parse().unwrap()));
        assert!(!is_public(&"10.17.3.215".parse().unwrap()));
        assert!(!is_public(&"192.168.1.5".parse().unwrap()));
        assert!(!is_public(&"100.64.0.1".parse().unwrap()));
        assert!(!is_public(&"127.0.0.1".parse().unwrap()));
    }
}
