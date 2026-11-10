use super::discord_ipc::{DiscordClient, Image, Link, RichPresence};
use super::game_activity::{self, GameEvent};
use super::game_details::{self, GameDetails};
use crate::state::AppState;
use gtk::glib;
use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const DISCORD_CLIENT_ID: &str = "363445589247131668";
const QUADRA_ICON_URL: &str = "https://raw.githubusercontent.com/QuadraHQ/quadra-rewrite/main/data/icons/org.quadrahq.Quadra.png";
const QUADRA_ICON_TEXT: &str = "Quadra";
const REFRESH_GAME_DETAILS_EVERY: Duration = Duration::from_secs(60);
const PROPERTIES_THAT_DECIDE_PRESENCE: [&str; 2] = ["running", "discord-rich-presence"];
const SHORTEST_DISCORD_TEXT: usize = 2;
const LONGEST_DISCORD_TEXT: usize = 128;
const INVISIBLE_PADDING: char = '\u{2800}';

struct CurrentGame {
    place_id: u64,
    joined_at: i64,
    details: Option<GameDetails>,
    refreshing_details: glib::SourceId,
}

#[derive(Default)]
struct Presence {
    discord: Option<DiscordClient>,
    roblox_started_at: Option<i64>,
    game: Option<CurrentGame>,
    shown_presence: Option<RichPresence>,
}

type SharedPresence = Rc<RefCell<Presence>>;

pub fn initialize(state: &AppState) {
    let presence = SharedPresence::default();

    state.connect_log_appended(glib::clone!(
        #[strong]
        presence,
        move |state, output| {
            for event in game_activity::events_in(output) {
                match event {
                    GameEvent::Joining { place_id, .. } => join_game(state, &presence, place_id),
                    GameEvent::Connected { .. } => {}
                    GameEvent::Left => leave_game(&presence),
                }
            }
            show(state, &presence);
        }
    ));

    for property in PROPERTIES_THAT_DECIDE_PRESENCE {
        state.connect_notify_local(
            Some(property),
            glib::clone!(
                #[strong]
                presence,
                move |state, _| {
                    follow_roblox(state, &presence);
                    show(state, &presence);
                }
            ),
        );
    }
}

fn follow_roblox(state: &AppState, presence: &SharedPresence) {
    if !state.running() {
        leave_game(presence);
        presence.borrow_mut().roblox_started_at = None;
        return;
    }

    presence
        .borrow_mut()
        .roblox_started_at
        .get_or_insert_with(now_in_seconds);

    let place_missing_details = presence
        .borrow()
        .game
        .as_ref()
        .filter(|game| game.details.is_none())
        .map(|game| game.place_id);
    if let Some(place_id) = place_missing_details {
        fetch_details(state, presence, place_id);
    }
}

fn join_game(state: &AppState, presence: &SharedPresence, place_id: u64) {
    let is_already_in_this_place = presence
        .borrow()
        .game
        .as_ref()
        .is_some_and(|game| game.place_id == place_id);
    if is_already_in_this_place {
        return;
    }
    leave_game(presence);

    let refreshing_details = glib::timeout_add_local(
        REFRESH_GAME_DETAILS_EVERY,
        glib::clone!(
            #[strong]
            state,
            #[strong]
            presence,
            move || {
                fetch_details(&state, &presence, place_id);
                glib::ControlFlow::Continue
            }
        ),
    );
    presence.borrow_mut().game = Some(CurrentGame {
        place_id,
        joined_at: now_in_seconds(),
        details: None,
        refreshing_details,
    });

    fetch_details(state, presence, place_id);
}

fn leave_game(presence: &SharedPresence) {
    let left_game = presence.borrow_mut().game.take();
    if let Some(game) = left_game {
        game.refreshing_details.remove();
    }
}

fn fetch_details(state: &AppState, presence: &SharedPresence, place_id: u64) {
    if !state.discord_rich_presence() {
        return;
    }

    let (details_sender, details_receiver) = async_channel::bounded(1);
    std::thread::spawn(move || {
        let _ = details_sender.send_blocking(game_details::fetch(place_id));
    });

    let state = state.clone();
    let presence = presence.clone();
    glib::spawn_future_local(async move {
        let fetched = match details_receiver.recv().await {
            Ok(Ok(details)) => details,
            Ok(Err(error)) => {
                eprintln!("Couldn't get the details of place {place_id}: {error:#}");
                return;
            }
            Err(_) => return,
        };

        if let Some(game) = presence
            .borrow_mut()
            .game
            .as_mut()
            .filter(|game| game.place_id == place_id)
        {
            let previous_icon_url = game
                .details
                .take()
                .and_then(|previous_details| previous_details.icon_url);
            game.details = Some(GameDetails {
                icon_url: fetched.icon_url.or(previous_icon_url),
                ..fetched
            });
        }
        show(&state, &presence);
    });
}

fn show(state: &AppState, presence: &SharedPresence) {
    let mut presence = presence.borrow_mut();
    let wants_presence = state.discord_rich_presence() && state.running();
    let rich_presence = wants_presence.then(|| match &presence.game {
        Some(game) => playing_presence(game),
        None => idle_presence(presence.roblox_started_at),
    });

    if presence.shown_presence == rich_presence {
        return;
    }
    presence.shown_presence = rich_presence.clone();
    presence
        .discord
        .get_or_insert_with(|| DiscordClient::start(DISCORD_CLIENT_ID))
        .show(rich_presence);
}

fn idle_presence(roblox_started_at: Option<i64>) -> RichPresence {
    RichPresence {
        details: "Idle".to_owned(),
        state: None,
        started_at: roblox_started_at,
        large_image: quadra_icon(),
        small_image: None,
        link: None,
    }
}

fn playing_presence(game: &CurrentGame) -> RichPresence {
    let game_page = Link {
        label: "See game page".to_owned(),
        url: format!("https://www.roblox.com/games/{}", game.place_id),
    };

    let Some(details) = &game.details else {
        return RichPresence {
            details: "Playing a game".to_owned(),
            state: None,
            started_at: Some(game.joined_at),
            large_image: quadra_icon(),
            small_image: None,
            link: Some(game_page),
        };
    };

    let game_name = discord_text(&details.name);
    let (large_image, small_image) = match &details.icon_url {
        Some(icon_url) => (
            Image {
                url: icon_url.clone(),
                text: game_name.clone(),
            },
            Some(quadra_icon()),
        ),
        None => (quadra_icon(), None),
    };

    RichPresence {
        details: game_name,
        state: Some(discord_text(&format!("by {}", details.creator))),
        started_at: Some(game.joined_at),
        large_image,
        small_image,
        link: Some(game_page),
    }
}

fn quadra_icon() -> Image {
    Image {
        url: QUADRA_ICON_URL.to_owned(),
        text: QUADRA_ICON_TEXT.to_owned(),
    }
}

fn discord_text(text: &str) -> String {
    let mut text: String = text.chars().take(LONGEST_DISCORD_TEXT).collect();
    while text.chars().count() < SHORTEST_DISCORD_TEXT {
        text.push(INVISIBLE_PADDING);
    }
    text
}

fn now_in_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|since_epoch| i64::try_from(since_epoch.as_secs()).ok())
        .unwrap_or_default()
}
