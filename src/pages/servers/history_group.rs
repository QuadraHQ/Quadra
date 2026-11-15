use super::join_availability::JoinAvailability;
use crate::backend;
use crate::state::{AppState, JoinedServer, PlayedGame};
use crate::utils::time_since;
use adw::prelude::*;
use gtk::glib;
use std::cell::RefCell;
use std::rc::Rc;

const UNKNOWN_LOCATION: &str = "Unknown location";

type ShownRows = Rc<RefCell<Vec<gtk::Widget>>>;

pub fn history_groups(
    state: &AppState,
    join_availability: &JoinAvailability,
) -> [adw::PreferencesGroup; 2] {
    let history_group = adw::PreferencesGroup::builder()
        .title("Server history")
        .description(
            "Servers you joined with Quadra. Rejoining asks Roblox for the same server, \
             and Roblox picks a new one if it has closed.",
        )
        .build();

    let forget_row = adw::ButtonRow::builder()
        .title("Clear history")
        .css_classes(["destructive-action"])
        .build();
    forget_row.connect_activated(glib::clone!(
        #[weak]
        state,
        move |_| backend::forget_server_history(&state)
    ));
    let forget_group = adw::PreferencesGroup::new();
    forget_group.add(&forget_row);

    let shown_rows = ShownRows::default();
    show_history(
        &history_group,
        &forget_group,
        &shown_rows,
        state,
        join_availability,
    );

    state.connect_server_history_notify(glib::clone!(
        #[weak]
        history_group,
        #[weak]
        forget_group,
        #[strong]
        shown_rows,
        #[strong]
        join_availability,
        move |state| show_history(
            &history_group,
            &forget_group,
            &shown_rows,
            state,
            &join_availability
        )
    ));

    [history_group, forget_group]
}

fn show_history(
    history_group: &adw::PreferencesGroup,
    forget_group: &adw::PreferencesGroup,
    shown_rows: &ShownRows,
    state: &AppState,
    join_availability: &JoinAvailability,
) {
    for row in shown_rows.borrow_mut().drain(..) {
        history_group.remove(&row);
    }

    let history = state.server_history();
    let rows: Vec<gtk::Widget> = if history.0.is_empty() {
        vec![nothing_played_row().upcast()]
    } else {
        history
            .0
            .iter()
            .map(|game| game_row(state, game, join_availability).upcast())
            .collect()
    };

    for row in &rows {
        history_group.add(row);
    }
    forget_group.set_visible(!history.0.is_empty());
    shown_rows.replace(rows);
}

fn nothing_played_row() -> adw::ActionRow {
    adw::ActionRow::builder()
        .title("No servers yet")
        .subtitle("Games you join will show up here.")
        .build()
}

fn game_row(
    state: &AppState,
    game: &PlayedGame,
    join_availability: &JoinAvailability,
) -> adw::ExpanderRow {
    let title = if game.name.is_empty() {
        format!("Place {}", game.place_id)
    } else {
        game.name.clone()
    };
    let row = adw::ExpanderRow::builder()
        .title(title)
        .subtitle(game.last_played_at().map(time_since).unwrap_or_default())
        .use_markup(false)
        .build();

    let place_id = game.place_id;
    let play_button = suffix_button("Play");
    play_button.add_css_class("suggested-action");
    play_button.connect_clicked(glib::clone!(
        #[weak]
        state,
        move |_| backend::join_game(&state, place_id)
    ));
    join_availability.applies_to(&play_button);
    row.add_suffix(&play_button);

    for server in &game.servers {
        row.add_row(&server_row(state, place_id, server, join_availability));
    }

    row
}

fn server_row(
    state: &AppState,
    place_id: u64,
    server: &JoinedServer,
    join_availability: &JoinAvailability,
) -> adw::ActionRow {
    let title = server
        .location
        .as_deref()
        .or(server.address.as_deref())
        .unwrap_or(UNKNOWN_LOCATION);
    let row = adw::ActionRow::builder()
        .title(title)
        .subtitle(time_since(server.joined_at))
        .use_markup(false)
        .build();

    let job_id = server.job_id.clone();
    let rejoin_button = suffix_button("Rejoin");
    rejoin_button.connect_clicked(glib::clone!(
        #[weak]
        state,
        move |_| backend::rejoin_server(&state, place_id, &job_id)
    ));
    join_availability.applies_to(&rejoin_button);
    row.add_suffix(&rejoin_button);

    row
}

fn suffix_button(label: &str) -> gtk::Button {
    gtk::Button::builder()
        .label(label)
        .valign(gtk::Align::Center)
        .build()
}
