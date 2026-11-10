use super::about::show_about;
use crate::backend;
use crate::state::AppState;
use adw::prelude::*;
use gtk::glib;

pub fn support_group(state: &AppState) -> adw::PreferencesGroup {
    let collect_button = gtk::Button::builder()
        .label("Collect")
        .valign(gtk::Align::Center)
        .build();
    collect_button.connect_clicked(glib::clone!(
        #[strong]
        state,
        move |_| backend::collect_diagnostics_in_background(&state)
    ));
    state
        .bind_property("busy", &collect_button, "sensitive")
        .transform_to(|_, busy: bool| Some(!busy))
        .sync_create()
        .build();

    let diagnostics_row = adw::ActionRow::builder()
        .title("Diagnostics")
        .subtitle("Save your settings, hardware details and recent Roblox log for a bug report. Cookies and join tickets are removed")
        .build();
    diagnostics_row.add_suffix(&collect_button);

    let about_row = adw::ButtonRow::builder().title("About Quadra").build();
    about_row.connect_activated(|row| show_about(row));

    let group = adw::PreferencesGroup::builder().title("Support").build();
    group.add(&diagnostics_row);
    group.add(&about_row);
    group
}
