use gtk::prelude::*;

pub fn app_icon(size: i32) -> gtk::Image {
    let icon = gtk::Image::from_file("data/icons/org.quadrahq.Quadra.png");
    icon.set_pixel_size(size);
    icon.add_css_class("app-icon");

    icon
}
