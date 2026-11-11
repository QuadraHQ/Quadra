use crate::backend;
use crate::state::AppState;
use adw::prelude::*;
use gtk::glib;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const PROPERTIES_THAT_DECIDE_UPDATE_ROW: [&str; 3] = ["update-status", "available-update", "busy"];

pub fn launcher_version_row(state: &AppState) -> adw::ActionRow {
    let check_button = gtk::Button::builder()
        .label("Check for updates")
        .valign(gtk::Align::Center)
        .build();
    check_button.connect_clicked(glib::clone!(
        #[strong]
        state,
        move |_| backend::check_for_updates(&state)
    ));

    let update_button = gtk::Button::builder()
        .label("Update")
        .css_classes(["suggested-action"])
        .valign(gtk::Align::Center)
        .build();
    update_button.connect_clicked(glib::clone!(
        #[strong]
        state,
        move |_| backend::install_update(&state)
    ));

    let row = adw::ActionRow::builder().title("Launcher version").build();
    row.add_suffix(&check_button);
    row.add_suffix(&update_button);

    state.follow_properties(
        &PROPERTIES_THAT_DECIDE_UPDATE_ROW,
        glib::clone!(
            #[weak]
            row,
            #[weak]
            check_button,
            #[weak]
            update_button,
            move |state| show_update_row(&row, &check_button, &update_button, state)
        ),
    );

    row
}

fn show_update_row(
    row: &adw::ActionRow,
    check_button: &gtk::Button,
    update_button: &gtk::Button,
    state: &AppState,
) {
    let status = state.update_status();
    row.set_subtitle(&if status.is_empty() {
        format!("v{VERSION}")
    } else {
        format!("v{VERSION}\n{status}")
    });

    let can_update_itself = backend::can_update_itself();
    check_button.set_visible(can_update_itself);
    check_button.set_sensitive(!state.busy());
    update_button.set_visible(can_update_itself && !state.available_update().is_empty());
    update_button.set_sensitive(!state.busy());
}
