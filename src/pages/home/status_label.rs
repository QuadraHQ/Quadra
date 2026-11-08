use crate::components::roblox_status::{PROPERTIES_THAT_DECIDE_STATUS, status_text};
use crate::state::{AppState, Client};
use adw::prelude::*;
use gtk::glib;

pub fn status_label(state: &AppState, client: Client) -> gtk::Label {
    let label = gtk::Label::builder()
        .css_classes(["dimmed"])
        .wrap(true)
        .justify(gtk::Justification::Center)
        .build();

    label.set_label(&status_text(state, client));
    for property in PROPERTIES_THAT_DECIDE_STATUS {
        state.connect_notify_local(
            Some(property),
            glib::clone!(
                #[weak]
                label,
                move |state, _| label.set_label(&status_text(state, client))
            ),
        );
    }

    label
}
