use crate::components::file_row::{Pick, file_row};
use crate::state::AppState;
use adw::prelude::*;

pub fn typography_group(state: &AppState) -> adw::PreferencesGroup {
    let group = adw::PreferencesGroup::builder().title("Typography").build();
    group.add(&file_row(
        state,
        "custom-font",
        "Custom font",
        &[Pick::File(&["ttf", "otf"])],
    ));
    group
}
