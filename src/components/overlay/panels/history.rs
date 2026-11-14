use crate::backend;
use crate::components::copy_button::copy_button;
use crate::state::{AppState, PlayedGame};
use crate::utils::time_since;
use adw::prelude::*;
use gtk::glib;

pub fn history_content(state: &AppState) -> gtk::Widget {
    let list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .valign(gtk::Align::Start)
        .build();
    show_games(&list, state);
    state.connect_server_history_notify(glib::clone!(
        #[weak]
        list,
        move |state| show_games(&list, state)
    ));

    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&list)
        .vexpand(true)
        .build();
    let body = gtk::Box::new(gtk::Orientation::Vertical, 0);
    body.add_css_class("overlay-body");
    body.append(&scroll);

    body.upcast()
}

fn show_games(list: &gtk::ListBox, state: &AppState) {
    list.remove_all();

    let history = state.server_history();
    if history.0.is_empty() {
        list.append(
            &adw::ActionRow::builder()
                .title("No games yet")
                .subtitle("Games you join will show up here.")
                .build(),
        );
    }
    for game in &history.0 {
        list.append(&game_row(game));
    }
}

fn game_row(game: &PlayedGame) -> adw::ActionRow {
    let title = if game.name.is_empty() {
        format!("Place {}", game.place_id)
    } else {
        game.name.clone()
    };
    let servers = match game.servers.len() {
        1 => "1 server".to_owned(),
        count => format!("{count} servers"),
    };
    let subtitle = match game.last_played_at() {
        Some(joined_at) => format!("{} · {servers}", time_since(joined_at)),
        None => servers,
    };
    let row = adw::ActionRow::builder()
        .title(glib::markup_escape_text(&title))
        .subtitle(glib::markup_escape_text(&subtitle))
        .build();

    let place_id = game.place_id;
    if let Some(latest) = game.servers.first() {
        let job_id = latest.job_id.clone();
        row.add_suffix(&copy_button("Copy join link", move || {
            backend::game_link(place_id, &job_id)
        }));
    }

    row
}
