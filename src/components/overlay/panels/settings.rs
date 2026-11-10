use crate::components::compositor_binding::compositor_binding_hint;
use crate::components::shortcut_recorder::shortcut_recorder;
use crate::state::AppState;
use gtk::prelude::*;

pub fn settings_content(state: &AppState) -> gtk::Widget {
    let body = gtk::Box::new(gtk::Orientation::Vertical, 14);
    body.add_css_class("overlay-body");

    body.append(&switch_row(
        state,
        "Show the overlay",
        "Turn the overlay off entirely.",
        "overlay-enabled",
    ));
    body.append(&switch_row(
        state,
        "Notify the server location",
        "Shows where your server is when you join.",
        "notify-server-location",
    ));

    let shortcut = gtk::Box::new(gtk::Orientation::Vertical, 6);
    let shortcut_title = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    shortcut_title.append(
        &gtk::Label::builder()
            .label("Overlay shortcut")
            .xalign(0.0)
            .hexpand(true)
            .build(),
    );
    shortcut_title.append(&shortcut_recorder(state));
    shortcut.append(&shortcut_title);
    shortcut.append(&compositor_binding_hint(state));
    body.append(&shortcut);

    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&body)
        .build();

    scroll.upcast()
}

fn switch_row(state: &AppState, title: &str, subtitle: &str, property: &str) -> gtk::Box {
    let text = gtk::Box::new(gtk::Orientation::Vertical, 2);
    text.set_hexpand(true);
    text.append(&gtk::Label::builder().label(title).xalign(0.0).build());
    text.append(
        &gtk::Label::builder()
            .label(subtitle)
            .xalign(0.0)
            .wrap(true)
            .css_classes(["dim-label", "caption"])
            .build(),
    );

    let switch = gtk::Switch::builder().valign(gtk::Align::Center).build();
    state
        .bind_property(property, &switch, "active")
        .bidirectional()
        .sync_create()
        .build();

    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    row.append(&text);
    row.append(&switch);
    row
}
