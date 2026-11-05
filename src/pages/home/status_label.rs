use crate::state::AppState;
use adw::prelude::*;
use gtk::glib;

const PROPERTIES_THAT_DECIDE_STATUS: [&str; 5] = [
    "busy",
    "task-description",
    "task-progress",
    "roblox-installed",
    "roblox-version",
];

pub fn status_label(state: &AppState) -> gtk::Label {
    let label = gtk::Label::builder()
        .css_classes(["dimmed"])
        .wrap(true)
        .justify(gtk::Justification::Center)
        .build();

    label.set_label(&status_text(state));
    for property in PROPERTIES_THAT_DECIDE_STATUS {
        state.connect_notify_local(
            Some(property),
            glib::clone!(
                #[weak]
                label,
                move |state, _| label.set_label(&status_text(state))
            ),
        );
    }

    label
}

fn status_text(state: &AppState) -> String {
    if state.busy() {
        let percent = (state.task_progress() * 100.0).round();
        if percent > 0.0 {
            format!("{} ({percent}%)", state.task_description())
        } else {
            format!("{}…", state.task_description())
        }
    } else if state.roblox_installed() {
        format!("Roblox {}", state.roblox_version())
    } else {
        "Roblox isn't installed".to_owned()
    }
}
