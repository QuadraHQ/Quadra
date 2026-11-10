use crate::backend;
use gtk::glib;
use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

const SAVE_DELAY_AFTER_LAST_CHANGE: Duration = Duration::from_millis(800);

pub fn notes_content() -> gtk::Widget {
    let notes = gtk::TextView::builder()
        .wrap_mode(gtk::WrapMode::WordChar)
        .top_margin(8)
        .bottom_margin(8)
        .left_margin(8)
        .right_margin(8)
        .build();
    notes.buffer().set_text(&backend::read_overlay_notes());

    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&notes)
        .vexpand(true)
        .build();
    let status = gtk::Label::builder()
        .xalign(1.0)
        .css_classes(["dim-label", "caption"])
        .build();

    let pending_save: Rc<RefCell<Option<glib::SourceId>>> = Rc::default();
    let save_now = {
        let notes = notes.clone();
        let status = status.clone();
        move || {
            let buffer = notes.buffer();
            let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
            if backend::write_overlay_notes(&text) {
                status.set_label(&saved_at_now());
            } else {
                status.set_label("Couldn't save the notes");
            }
        }
    };

    notes.buffer().connect_changed({
        let pending_save = pending_save.clone();
        let save_now = save_now.clone();
        move |_| {
            if let Some(previous_save) = pending_save.take() {
                previous_save.remove();
            }
            let pending_save_after_delay = pending_save.clone();
            let save_now = save_now.clone();
            pending_save.replace(Some(glib::timeout_add_local_once(
                SAVE_DELAY_AFTER_LAST_CHANGE,
                move || {
                    pending_save_after_delay.take();
                    save_now();
                },
            )));
        }
    });
    notes.connect_unmap(move |_| {
        if let Some(pending) = pending_save.take() {
            pending.remove();
            save_now();
        }
    });

    let body = gtk::Box::new(gtk::Orientation::Vertical, 6);
    body.add_css_class("overlay-body");
    body.append(&scroll);
    body.append(&status);

    body.upcast()
}

fn saved_at_now() -> String {
    glib::DateTime::now_local()
        .and_then(|now| now.format("%-I:%M %p"))
        .map_or_else(|_| "Saved".to_owned(), |time| format!("Saved at {time}"))
}
