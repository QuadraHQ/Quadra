use crate::backend;
use crate::state::AppState;
use adw::prelude::*;
use gtk::glib;

const DESCRIPTION: &str =
    "Programs that start Rosemary, like gamemoderun or gamescope -f --. They can be chained.";

pub fn launch_wrapper_group(state: &AppState) -> adw::PreferencesGroup {
    let row = adw::EntryRow::builder().title("Launch wrapper").build();
    state
        .bind_property("launch-wrapper", &row, "text")
        .bidirectional()
        .sync_create()
        .build();

    let group = adw::PreferencesGroup::builder()
        .title("Launch wrapper")
        .description(DESCRIPTION)
        .build();
    group.add(&row);

    state.follow_properties(
        &["launch-wrapper"],
        glib::clone!(
            #[weak]
            row,
            #[weak]
            group,
            move |state| show_launch_wrapper_problem(&row, &group, state)
        ),
    );

    group
}

fn show_launch_wrapper_problem(
    row: &adw::EntryRow,
    group: &adw::PreferencesGroup,
    state: &AppState,
) {
    match backend::launch_wrapper_problem(&state.launch_wrapper()) {
        Some(problem) => {
            row.add_css_class("error");
            group.set_description(Some(&problem));
        }
        None => {
            row.remove_css_class("error");
            group.set_description(Some(DESCRIPTION));
        }
    }
}
