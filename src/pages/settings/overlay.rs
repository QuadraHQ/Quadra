use crate::components::compositor_binding::compositor_binding_hint;
use crate::components::overlay::is_supported;
use crate::components::shortcut_recorder::shortcut_recorder;
use crate::state::AppState;
use adw::prelude::*;

pub fn overlay_section(state: &AppState) -> gtk::Box {
    let group = adw::PreferencesGroup::builder()
        .title("Overlay")
        .description(
            "Panels that float above Roblox while you play. The overlay is a separate window \
             drawn by your desktop, so it never touches Roblox's memory.",
        )
        .build();

    let enabled = adw::SwitchRow::builder()
        .title("Enable the overlay")
        .subtitle(if is_supported() {
            "Open it with the shortcut below or from the tray menu."
        } else {
            "Your desktop doesn't support it: the overlay needs the Wayland layer-shell protocol."
        })
        .sensitive(is_supported())
        .build();
    state
        .bind_property("overlay-enabled", &enabled, "active")
        .bidirectional()
        .sync_create()
        .build();
    group.add(&enabled);

    let shortcut = adw::ActionRow::builder()
        .title("Overlay shortcut")
        .subtitle("Closes the overlay while it has focus. Opening it needs a bind in your desktop.")
        .build();
    shortcut.add_suffix(&shortcut_recorder(state));
    group.add(&shortcut);

    let binding = gtk::Box::new(gtk::Orientation::Vertical, 0);
    binding.set_margin_top(12);
    binding.set_margin_bottom(12);
    binding.set_margin_start(12);
    binding.set_margin_end(12);
    binding.append(&compositor_binding_hint(state));
    let binding_row = adw::PreferencesRow::new();
    binding_row.set_child(Some(&binding));
    binding_row.set_activatable(false);
    group.add(&binding_row);

    let section = gtk::Box::new(gtk::Orientation::Vertical, 12);
    section.append(&group);

    section
}
