use super::logs_button::logs_button;
use super::play_button::play_button;
use super::status_label::status_label;
use crate::components::icon::app_icon;
use crate::state::{AppState, Client};
use adw::prelude::*;

pub fn client_page(state: &AppState, client: Client) -> gtk::Widget {
    let logo = app_icon(128);

    let title = gtk::Label::builder()
        .label(client.home_title())
        .css_classes(["title-1"])
        .build();

    let subtitle = status_label(state, client);

    let logs_button = logs_button(state);

    let play_button = play_button(state, client);

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
    content.append(&logs_button);
    content.append(&play_button);

    content.upcast()
}
