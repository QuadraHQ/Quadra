use crate::backend;
use crate::components::command_dialog::show_command_dialog;
use adw::prelude::*;

pub fn butter_access_row() -> Option<adw::ActionRow> {
    let command = backend::missing_butter_access()?;

    let button = gtk::Button::builder()
        .label("Show how")
        .valign(gtk::Align::Center)
        .build();
    let row = adw::ActionRow::builder()
        .title("Looking for Butter runners?")
        .subtitle("Quadra runs as a Flatpak, so it can only list the runners Butter installed once you allow it.")
        .build();
    row.add_suffix(&button);

    button.connect_clicked(move |button| {
        show_command_dialog(
            button,
            "Let Quadra see Butter runners",
            "Run this in a terminal to let Quadra read the runners Butter installed, then restart Quadra:",
            &command,
        );
    });

    Some(row)
}
