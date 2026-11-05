use crate::components::icon::app_icon;
use crate::state::AppState;
use adw::prelude::*;

pub fn home_page(state: &AppState) -> gtk::Widget {
    let logo = app_icon(128);

    let title = gtk::Label::builder()
        .label("Quadra")
        .css_classes(["title-1"])
        .build();

    let subtitle = gtk::Label::builder()
        .label("Roblox not found")
        .css_classes(["dimmed"])
        .wrap(true)
        .justify(gtk::Justification::Center)
        .build();

    let play_button = gtk::Button::builder()
        .child(
            &adw::ButtonContent::builder()
                .label("Play")
                .icon_name("media-playback-start-symbolic")
                .build(),
        )
        .css_classes(["suggested-action", "pill"])
        .halign(gtk::Align::Center)
        .margin_top(12)
        .build();
    play_button.connect_clicked(|_| println!("play clicked"));

    state
        .bind_property("running", &play_button, "sensitive")
        .invert_boolean()
        .sync_create()
        .build();

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
