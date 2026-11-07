use super::join_availability::JoinAvailability;
use super::place_link::place_id_in;
use crate::backend;
use crate::state::AppState;
use adw::prelude::*;
use gtk::glib;

const INVALID_INPUT_STYLE: &str = "error";

pub fn join_by_link_row(state: &AppState, join_availability: &JoinAvailability) -> adw::EntryRow {
    let row = adw::EntryRow::builder()
        .title("Join a game by ID or roblox.com link")
        .show_apply_button(true)
        .build();

    row.connect_apply(glib::clone!(
        #[strong]
        state,
        move |row| match place_id_in(&row.text()) {
            Some(place_id) => {
                row.remove_css_class(INVALID_INPUT_STYLE);
                row.set_text("");
                backend::join_game(&state, place_id);
            }
            None => row.add_css_class(INVALID_INPUT_STYLE),
        }
    ));
    row.connect_changed(|row| row.remove_css_class(INVALID_INPUT_STYLE));

    join_availability.applies_to(&row);
    row
}
