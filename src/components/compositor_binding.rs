use super::copy_button::copy_button;
use crate::backend;
use crate::command_line::toggle_overlay_command;
use crate::state::AppState;
use gtk::glib;
use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

pub fn compositor_binding_hint(state: &AppState) -> gtk::Box {
    let description = gtk::Label::builder()
        .xalign(0.0)
        .wrap(true)
        .css_classes(["dim-label", "caption"])
        .build();
    let line = gtk::Label::builder()
        .xalign(0.0)
        .hexpand(true)
        .wrap(true)
        .wrap_mode(gtk::pango::WrapMode::Word)
        .css_classes(["overlay-command"])
        .build();

    let copied_line = Rc::new(RefCell::new(String::new()));
    let hint = gtk::Box::new(gtk::Orientation::Vertical, 4);
    hint.append(&description);
    let line_row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    line_row.append(&line);
    line_row.append(&copy_button("Copy", {
        let copied_line = copied_line.clone();
        move || copied_line.borrow().clone()
    }));
    hint.append(&line_row);

    let show_binding = glib::clone!(
        #[weak]
        description,
        #[weak]
        line,
        move |state: &AppState| {
            let (explanation, text) =
                match backend::compositor_binding_for(&state.overlay_shortcut()) {
                    Some(binding) => (
                        format!(
                            "Your desktop can't give Quadra a global shortcut. To open the overlay \
                         while Roblox is focused, add this to the binds of your {} config:",
                            binding.compositor
                        ),
                        binding.line,
                    ),
                    None => (
                        "To open the overlay while Roblox is focused, bind this command to your \
                     shortcut in your desktop's keyboard settings:"
                            .to_owned(),
                        toggle_overlay_command(),
                    ),
                };
            description.set_label(&explanation);
            line.set_label(&text);
            copied_line.replace(text);
        }
    );
    show_binding(state);
    state.connect_overlay_shortcut_notify(show_binding);

    hint
}
