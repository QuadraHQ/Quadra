mod flag_rows;
mod transfer;

use crate::state::AppState;
use adw::prelude::*;
use flag_rows::{FlagRows, add_flag, show_custom_flags};
use glib::clone;
use gtk::glib;

pub fn custom_flags_group(state: &AppState) -> adw::PreferencesGroup {
    let add_button = header_button("Add a flag", Some("list-add-symbolic"), None);
    let import_button = header_button("Import flags from JSON", None, Some("Import"));
    let export_button = header_button("Export flags as JSON", None, Some("Export"));

    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    buttons.append(&add_button);
    buttons.append(&import_button);
    buttons.append(&export_button);

    let group = adw::PreferencesGroup::builder()
        .title("Custom flags")
        .description(
            "Any other FastFlag, written as NAME=value. Roblox only honours allowlisted flags.",
        )
        .header_suffix(&buttons)
        .build();

    let rows = FlagRows::default();
    show_custom_flags(&group, &rows, state);

    add_button.connect_clicked(clone!(
        #[weak]
        group,
        #[strong]
        rows,
        #[strong]
        state,
        move |_| add_flag(&group, &rows, &state)
    ));
    import_button.connect_clicked(clone!(
        #[weak]
        group,
        #[strong]
        rows,
        #[strong]
        state,
        move |button| {
            let (group, rows) = (group.clone(), rows.clone());
            transfer::show_import(button, &state, move |state| {
                show_custom_flags(&group, &rows, state)
            });
        }
    ));
    export_button.connect_clicked(clone!(
        #[strong]
        state,
        move |button| transfer::show_export(button, &state)
    ));

    group
}

fn header_button(tooltip: &str, icon: Option<&str>, label: Option<&str>) -> gtk::Button {
    let button = gtk::Button::builder()
        .tooltip_text(tooltip)
        .css_classes(["flat"])
        .valign(gtk::Align::Center)
        .build();
    if let Some(icon) = icon {
        button.set_icon_name(icon);
    }
    if let Some(label) = label {
        button.set_label(label);
    }
    button
}
