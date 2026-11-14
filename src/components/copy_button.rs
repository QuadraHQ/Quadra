use gtk::prelude::*;

pub fn copy_button(tooltip: &str, text: impl Fn() -> String + 'static) -> gtk::Button {
    let button = gtk::Button::builder()
        .icon_name("edit-copy-symbolic")
        .tooltip_text(tooltip)
        .valign(gtk::Align::Center)
        .css_classes(["flat", "circular"])
        .build();
    copy_on_click(&button, text);
    button
}

pub fn copy_on_click(button: &gtk::Button, text: impl Fn() -> String + 'static) {
    button.connect_clicked(move |button| button.clipboard().set_text(&text()));
}
