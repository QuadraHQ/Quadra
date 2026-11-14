use gtk::glib;
use gtk::prelude::*;

pub fn every_second_while_shown<W: IsA<gtk::Widget>>(widget: &W, update: impl Fn(&W) + 'static) {
    update(widget);
    let widget = widget.downgrade();
    glib::timeout_add_seconds_local(1, move || {
        let Some(widget) = widget.upgrade() else {
            return glib::ControlFlow::Break;
        };
        if widget.is_mapped() {
            update(&widget);
        }
        glib::ControlFlow::Continue
    });
}
