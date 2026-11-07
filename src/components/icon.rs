use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;

const APP_ICON_PNG: &[u8] = include_bytes!("../../data/icons/org.quadrahq.Quadra.png");

pub fn app_icon_texture() -> Result<gdk::Texture, glib::Error> {
    gdk::Texture::from_bytes(&glib::Bytes::from_static(APP_ICON_PNG))
}

pub fn app_icon(size: i32) -> gtk::Image {
    let icon = gtk::Image::new();
    match app_icon_texture() {
        Ok(texture) => icon.set_paintable(Some(&texture)),
        Err(error) => eprintln!("Failed to load the app icon: {error}"),
    }
    icon.set_pixel_size(size);
    icon.add_css_class("app-icon");

    icon
}
