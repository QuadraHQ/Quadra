use crate::state::AppState;
use adw::prelude::*;
use gtk::glib;

pub fn play_button(state: &AppState) -> gtk::Button {
    let content = adw::ButtonContent::new();

    let button = gtk::Button::builder()
        .child(&content)
        .css_classes(["pill"])
        .halign(gtk::Align::Center)
        .margin_top(12)
        .build();

    button.connect_clicked(glib::clone!(
        #[strong]
        state,
        move |_| {
            if state.running() {
                println!("stop clicked");
            } else {
                println!("play clicked");
            }
        }
    ));

    state
        .bind_property("can-play", &button, "sensitive")
        .sync_create()
        .build();

    show_play_or_stop(&button, &content, state.running());
    state.connect_running_notify(glib::clone!(
        #[weak]
        button,
        #[weak]
        content,
        move |state| show_play_or_stop(&button, &content, state.running())
    ));

    button
}

fn show_play_or_stop(button: &gtk::Button, content: &adw::ButtonContent, running: bool) {
    if running {
        content.set_label("Stop");
        content.set_icon_name("media-playback-stop-symbolic");
        button.remove_css_class("suggested-action");
        button.add_css_class("destructive-action");
    } else {
        content.set_label("Play");
        content.set_icon_name("media-playback-start-symbolic");
        button.remove_css_class("destructive-action");
        button.add_css_class("suggested-action");
    }
}
