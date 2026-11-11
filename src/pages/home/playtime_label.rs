use crate::state::AppState;
use crate::utils::format_playtime;
use adw::prelude::*;
use gtk::glib;

const PROPERTIES_THAT_DECIDE_PLAYTIME: [&str; 2] = ["show-playtime", "playtime-seconds"];

pub fn playtime_label(state: &AppState) -> gtk::Label {
    let label = gtk::Label::builder().css_classes(["dimmed"]).build();

    state.follow_properties(
        &PROPERTIES_THAT_DECIDE_PLAYTIME,
        glib::clone!(
            #[weak]
            label,
            move |state| show_playtime(&label, state)
        ),
    );

    label
}

fn show_playtime(label: &gtk::Label, state: &AppState) {
    let seconds = state.playtime_seconds();
    label.set_visible(state.show_playtime() && seconds > 0);
    label.set_label(&format!("Played for {}", format_playtime(seconds)));
}
