use crate::state::{AppState, DEFAULT_OVERLAY_SHORTCUT};
use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;
use std::cell::Cell;
use std::rc::Rc;

const RECORDING_LABEL: &str = "Press a shortcut…";
const NEEDS_MODIFIER_LABEL: &str = "Include Ctrl, Alt or Super";
const NOT_SET_LABEL: &str = "Not set";

pub fn is_pressed(accelerator: &str, key: gdk::Key, modifiers: gdk::ModifierType) -> bool {
    let Some((wanted_key, wanted_modifiers)) = gtk::accelerator_parse(accelerator) else {
        return false;
    };
    key.to_lower() == wanted_key.to_lower()
        && modifiers & gtk::accelerator_get_default_mod_mask() == wanted_modifiers
}

pub fn label_of(accelerator: &str) -> String {
    match gtk::accelerator_parse(accelerator) {
        Some((key, modifiers)) => gtk::accelerator_get_label(key, modifiers).to_string(),
        None => NOT_SET_LABEL.to_owned(),
    }
}

pub fn shortcut_recorder(state: &AppState) -> gtk::Box {
    let button = gtk::Button::builder()
        .valign(gtk::Align::Center)
        .label(label_of(&state.overlay_shortcut()))
        .tooltip_text("Click, then press the keys you want")
        .build();
    let reset = gtk::Button::builder()
        .icon_name("edit-undo-symbolic")
        .tooltip_text(format!("Reset to {}", label_of(DEFAULT_OVERLAY_SHORTCUT)))
        .valign(gtk::Align::Center)
        .css_classes(["flat", "circular"])
        .build();
    let recording = Rc::new(Cell::new(false));

    let show_shortcut = {
        let button = button.clone();
        let reset = reset.clone();
        move |state: &AppState| {
            let shortcut = state.overlay_shortcut();
            button.set_label(&label_of(&shortcut));
            reset.set_sensitive(shortcut != DEFAULT_OVERLAY_SHORTCUT);
        }
    };
    show_shortcut(state);
    state.connect_overlay_shortcut_notify(show_shortcut.clone());

    button.connect_clicked({
        let recording = recording.clone();
        move |button| {
            recording.set(true);
            button.set_label(RECORDING_LABEL);
            button.grab_focus();
        }
    });
    reset.connect_clicked(glib::clone!(
        #[strong]
        state,
        move |_| state.set_overlay_shortcut(DEFAULT_OVERLAY_SHORTCUT)
    ));

    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    keys.connect_key_pressed(glib::clone!(
        #[strong]
        state,
        #[strong]
        recording,
        #[strong]
        show_shortcut,
        #[weak]
        button,
        #[upgrade_or]
        glib::Propagation::Proceed,
        move |_, key, _, modifiers| {
            if !recording.get() {
                return glib::Propagation::Proceed;
            }
            let modifiers = modifiers & gtk::accelerator_get_default_mod_mask();
            if is_modifier(key) {
                return glib::Propagation::Stop;
            }
            if key == gdk::Key::Escape && modifiers.is_empty() {
                recording.set(false);
                show_shortcut(&state);
                return glib::Propagation::Stop;
            }
            let has_primary_modifier = modifiers.intersects(
                gdk::ModifierType::CONTROL_MASK
                    | gdk::ModifierType::ALT_MASK
                    | gdk::ModifierType::SUPER_MASK,
            );
            if !has_primary_modifier {
                button.set_label(NEEDS_MODIFIER_LABEL);
                return glib::Propagation::Stop;
            }
            recording.set(false);
            state.set_overlay_shortcut(gtk::accelerator_name(key.to_lower(), modifiers).as_str());
            show_shortcut(&state);
            glib::Propagation::Stop
        }
    ));
    button.add_controller(keys);

    let focus = gtk::EventControllerFocus::new();
    focus.connect_leave(glib::clone!(
        #[strong]
        state,
        #[strong]
        recording,
        move |_| {
            if recording.replace(false) {
                show_shortcut(&state);
            }
        }
    ));
    button.add_controller(focus);

    let recorder = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    recorder.append(&button);
    recorder.append(&reset);
    recorder
}

fn is_modifier(key: gdk::Key) -> bool {
    [
        gdk::Key::Control_L,
        gdk::Key::Control_R,
        gdk::Key::Alt_L,
        gdk::Key::Alt_R,
        gdk::Key::Shift_L,
        gdk::Key::Shift_R,
        gdk::Key::Super_L,
        gdk::Key::Super_R,
        gdk::Key::Meta_L,
        gdk::Key::Meta_R,
        gdk::Key::Hyper_L,
        gdk::Key::Hyper_R,
        gdk::Key::ISO_Level3_Shift,
        gdk::Key::Caps_Lock,
    ]
    .contains(&key)
}
