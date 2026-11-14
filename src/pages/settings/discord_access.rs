use crate::backend::{self, MissingDiscordAccess};
use adw::prelude::*;

const COPY_RESPONSE: &str = "copy";
const CLOSE_RESPONSE: &str = "close";

pub fn ask_for_discord_access_when_turned_on(row: &adw::SwitchRow) {
    row.connect_active_notify(|row| {
        if row.is_active()
            && let Some(missing) = backend::missing_discord_access()
        {
            show_discord_access_dialog(row, missing);
        }
    });
}

fn show_discord_access_dialog(row: &adw::SwitchRow, missing: MissingDiscordAccess) {
    let found = if missing.discords.is_empty() {
        "Quadra couldn't find Discord running, so this allows every Discord app it supports. \
         To allow only yours, open Discord and turn this on again."
            .to_owned()
    } else {
        format!(
            "Quadra found {} running, but runs as a Flatpak, so it can't reach it yet.",
            missing.discords.join(" and ")
        )
    };

    let dialog = adw::AlertDialog::builder()
        .heading("Let Quadra reach Discord")
        .prefer_wide_layout(true)
        .body(format!(
            "{found} Run this in a terminal to allow it, then restart Quadra:"
        ))
        .extra_child(&code_block(&missing.command))
        .build();
    dialog.add_response(CLOSE_RESPONSE, "Close");
    dialog.add_response(COPY_RESPONSE, "Copy command");
    dialog.set_response_appearance(COPY_RESPONSE, adw::ResponseAppearance::Suggested);
    dialog.set_default_response(Some(COPY_RESPONSE));
    dialog.set_close_response(CLOSE_RESPONSE);

    let command = missing.command;
    dialog.connect_response(Some(COPY_RESPONSE), move |dialog, _| {
        dialog.clipboard().set_text(&command);
    });

    dialog.present(Some(row));
}

fn code_block(code: &str) -> gtk::Box {
    const PADDING: i32 = 12;
    let label = gtk::Label::builder()
        .label(code)
        .selectable(true)
        .wrap(true)
        .wrap_mode(gtk::pango::WrapMode::Char)
        .xalign(0.0)
        .css_classes(["monospace"])
        .margin_top(PADDING)
        .margin_bottom(PADDING)
        .margin_start(PADDING)
        .margin_end(PADDING)
        .build();
    let block = gtk::Box::builder().css_classes(["card"]).build();
    block.append(&label);
    block
}
