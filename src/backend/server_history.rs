use super::game_activity::{self, GameEvent};
use super::game_details;
use super::paths;
use super::regions::PickedServer;
use crate::state::{AppState, JoinedServer, ServerHistory};
use gtk::glib;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::io::ErrorKind;
use std::time::{SystemTime, UNIX_EPOCH};

thread_local! {
    static LOCATIONS_OF_PICKED_SERVERS: RefCell<HashMap<String, String>> = RefCell::default();
}

pub fn initialize(state: &AppState) {
    state.set_server_history(read());

    state.connect_log_appended(|state, output| {
        for event in game_activity::events_in(output) {
            if let GameEvent::Joining {
                place_id,
                job_id,
                server_address,
            } = event
            {
                remember_join(state, place_id, job_id, server_address);
            }
        }
    });
}

pub fn expect_join_to(picked_server: PickedServer) {
    if let Some(location) = picked_server.location {
        LOCATIONS_OF_PICKED_SERVERS
            .with_borrow_mut(|locations| locations.insert(picked_server.job_id, location));
    }
}

pub fn remember_location(state: &AppState, job_id: &str, location: &str) {
    save(
        state,
        state.server_history().with_location(job_id, location),
    );
}

pub fn forget_all(state: &AppState) {
    save(state, ServerHistory::default());
}

fn remember_join(state: &AppState, place_id: u64, job_id: String, server_address: Option<String>) {
    let location =
        LOCATIONS_OF_PICKED_SERVERS.with_borrow_mut(|locations| locations.remove(&job_id));
    let joined_server = JoinedServer {
        job_id,
        address: server_address,
        location,
        joined_at: seconds_since_epoch(),
    };
    save(
        state,
        state.server_history().with_join(place_id, joined_server),
    );

    if !state.server_history().has_name_for(place_id) {
        fetch_game_name(state, place_id);
    }
}

fn fetch_game_name(state: &AppState, place_id: u64) {
    let (name_sender, name_receiver) = async_channel::bounded(1);
    std::thread::spawn(move || {
        let _ = name_sender.send_blocking(game_details::fetch(place_id));
    });

    let state = state.clone();
    glib::spawn_future_local(async move {
        match name_receiver.recv().await {
            Ok(Ok(details)) => save(
                &state,
                state
                    .server_history()
                    .with_game_name(place_id, &details.name),
            ),
            Ok(Err(error)) => eprintln!("Couldn't get the name of place {place_id}: {error:#}"),
            Err(_) => {}
        }
    });
}

fn save(state: &AppState, history: ServerHistory) {
    let path = paths::server_history_file();
    let written = path
        .parent()
        .map_or(Ok(()), fs::create_dir_all)
        .map_err(anyhow::Error::from)
        .and_then(|()| Ok(serde_json::to_string_pretty(&history)?))
        .and_then(|contents| {
            let unfinished_path = path.with_extension("json.partial");
            fs::write(&unfinished_path, contents)?;
            Ok(fs::rename(&unfinished_path, &path)?)
        });
    if let Err(error) = written {
        eprintln!(
            "Couldn't save the server history to {}: {error:#}",
            path.display()
        );
    }

    state.set_server_history(history);
}

fn read() -> ServerHistory {
    let path = paths::server_history_file();
    match fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_else(|error| {
            eprintln!(
                "Ignoring the invalid server history in {}: {error}",
                path.display()
            );
            ServerHistory::default()
        }),
        Err(error) if error.kind() == ErrorKind::NotFound => ServerHistory::default(),
        Err(error) => {
            eprintln!(
                "Couldn't read the server history from {}: {error}",
                path.display()
            );
            ServerHistory::default()
        }
    }
}

fn seconds_since_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|since_epoch| i64::try_from(since_epoch.as_secs()).ok())
        .unwrap_or_default()
}
