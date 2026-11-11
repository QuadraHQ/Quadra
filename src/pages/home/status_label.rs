use crate::components::roblox_status::{PROPERTIES_THAT_DECIDE_STATUS, status_text};
use crate::state::{AppState, Client};
use gtk::glib;

pub fn status_label(state: &AppState, client: Client) -> gtk::Label {
    let label = gtk::Label::builder()
        .css_classes(["dimmed"])
        .wrap(true)
        .justify(gtk::Justification::Center)
        .build();

    state.follow_properties(
        &PROPERTIES_THAT_DECIDE_STATUS,
        glib::clone!(
            #[weak]
            label,
            move |state| label.set_label(&status_text(state, client))
        ),
    );

    label
}
