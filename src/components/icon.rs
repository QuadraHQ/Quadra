use gtk::prelude::*;

pub const APP_ICON_PATH: &str = "data/icons/org.quadrahq.Quadra.png";

pub fn app_icon(size: i32) -> gtk::Image {
    let icon = gtk::Image::from_file(APP_ICON_PATH);
    icon.set_pixel_size(size);
    icon.add_css_class("app-icon");

    icon
}
