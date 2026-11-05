mod play_button;
mod status_label;

use crate::components::icon::app_icon;
use crate::state::AppState;
use adw::prelude::*;
use play_button::play_button;
use status_label::status_label;

pub fn home_page(state: &AppState) -> gtk::Widget {
    let logo = app_icon(128);

    let title = gtk::Label::builder()
        .label("Quadra")
        .css_classes(["title-1"])
        .build();

    let subtitle = status_label(state);

    let play_button = play_button(state);

    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .vexpand(true)
        .build();
    content.append(&logo);
    content.append(&title);
    content.append(&subtitle);
    content.append(&play_button);

    content.upcast()
}
