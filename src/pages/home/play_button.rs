use crate::state::AppState;
use adw::prelude::*;

pub fn play_button(state: &AppState) -> gtk::Button {
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

    play_button
}
