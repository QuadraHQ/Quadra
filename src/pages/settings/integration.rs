use crate::backend::{self, Handler, Shortcut};
use crate::state::AppState;
use adw::prelude::*;
use gtk::glib;

pub fn integration_section(state: &AppState) -> gtk::Box {
    let group = adw::PreferencesGroup::builder()
        .title("App menu")
        .description(
            "Adds entries to your app menu. The Roblox and Roblox Studio entries start them \
             straight away, showing only their progress while they open.",
        )
        .build();
    for shortcut in Shortcut::ALL {
        group.add(&shortcut_row(state, shortcut));
    }

    let handlers_group = adw::PreferencesGroup::builder()
        .title("Links and files")
        .description("Makes Quadra the app your system uses for these.")
        .build();
    for handler in Handler::ALL {
        handlers_group.add(&handler_row(state, handler));
    }

    let section = gtk::Box::new(gtk::Orientation::Vertical, 12);
    section.append(&group);
    section.append(&handlers_group);

    section
}

fn handler_row(state: &AppState, handler: Handler) -> adw::SwitchRow {
    let row = adw::SwitchRow::builder()
        .title(handler.title())
        .subtitle(handler.description())
        .active(backend::handler_is_enabled(handler))
        .build();

    row.connect_active_notify(glib::clone!(
        #[strong]
        state,
        move |row| {
            if row.is_active() == backend::handler_is_enabled(handler) {
                return;
            }
            backend::set_handler_enabled(&state, handler, row.is_active());
            row.set_active(backend::handler_is_enabled(handler));
        }
    ));

    row
}

fn shortcut_row(state: &AppState, shortcut: Shortcut) -> adw::ActionRow {
    let button = gtk::Button::builder().valign(gtk::Align::Center).build();

    let entry = shortcut.entry();
    let row = adw::ActionRow::builder()
        .title(entry.name)
        .subtitle(entry.description)
        .build();
    row.add_suffix(&button);

    button.connect_clicked(glib::clone!(
        #[strong]
        state,
        move |button| {
            if backend::has_shortcut(shortcut) {
                backend::remove_shortcut(&state, shortcut);
            } else {
                backend::add_shortcut(&state, shortcut);
            }
            show_whether_added(button, shortcut);
        }
    ));
    show_whether_added(&button, shortcut);

    row
}

fn show_whether_added(button: &gtk::Button, shortcut: Shortcut) {
    if backend::has_shortcut(shortcut) {
        button.set_label("Remove");
        button.remove_css_class("suggested-action");
    } else {
        button.set_label("Add");
        button.add_css_class("suggested-action");
    }
}
