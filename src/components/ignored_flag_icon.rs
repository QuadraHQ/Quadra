use crate::backend;
use gtk::prelude::*;

pub fn ignored_flag_icon() -> gtk::Image {
    gtk::Image::builder()
        .icon_name("dialog-warning-symbolic")
        .tooltip_text("Roblox ignores this FastFlag, so it has no effect")
        .valign(gtk::Align::Center)
        .visible(false)
        .build()
}

pub fn show_if_ignored(icon: &gtk::Image, name: &str) {
    let name = name.trim();
    icon.set_visible(!name.is_empty() && backend::flag_is_ignored(name));
}
