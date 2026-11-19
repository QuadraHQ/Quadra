use super::about::show_about;
use crate::backend;
use crate::state::AppState;
use adw::prelude::*;
use gtk::glib;

const BYTES_PER_MEGABYTE: u64 = 1_000_000;
const CACHE_SUBTITLE: &str = "Deletes the downloaded interface updates and assets. Your sign-in is kept, and the next launch downloads them again";

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

    let cache_button = gtk::Button::builder()
        .label("Clear")
        .valign(gtk::Align::Center)
        .build();
    let cache_row = adw::ActionRow::builder()
        .title("Roblox cache")
        .subtitle(CACHE_SUBTITLE)
        .build();
    cache_row.add_suffix(&cache_button);
    cache_button.connect_clicked(glib::clone!(
        #[strong]
        state,
        #[weak]
        cache_row,
        move |_| backend::clear_roblox_cache(&state, move |freed| {
            cache_row.set_subtitle(&format!("Cleared {} MB", freed / BYTES_PER_MEGABYTE));
        })
    ));
    state
        .bind_property("busy", &cache_button, "sensitive")
        .transform_to(|_, busy: bool| Some(!busy))
        .sync_create()
        .build();

    let about_row = adw::ButtonRow::builder().title("About Quadra").build();
    about_row.connect_activated(show_about);

    let group = adw::PreferencesGroup::builder().title("Support").build();
    group.add(&diagnostics_row);
    group.add(&cache_row);
    group.add(&about_row);
    group
}
