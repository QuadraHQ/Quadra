use crate::state::AppState;
use crate::utils::path_for_display;
use adw::prelude::*;
use gtk::{gio, glib};
use std::path::PathBuf;

const NOTHING_CHOSEN: &str = "Nothing chosen";

#[derive(Clone, Copy)]
pub enum Pick {
    File(&'static [&'static str]),
    Folder,
}

pub fn file_row(
    state: &AppState,
    property: &'static str,
    title: &str,
    picks: &[Pick],
) -> adw::ActionRow {
    let row = adw::ActionRow::builder().title(title).build();

    state
        .bind_property(property, &row, "subtitle")
        .transform_to(|_, path: String| Some(shown(&path)))
        .sync_create()
        .build();

    let clear_button = gtk::Button::builder()
        .icon_name("edit-clear-symbolic")
        .tooltip_text("Clear")
        .css_classes(["flat"])
        .valign(gtk::Align::Center)
        .build();
    clear_button.connect_clicked(glib::clone!(
        #[weak]
        state,
        move |_| state.set_property(property, "")
    ));
    state
        .bind_property(property, &clear_button, "visible")
        .transform_to(|_, path: String| Some(!path.is_empty()))
        .sync_create()
        .build();
    row.add_suffix(&clear_button);

    for pick in picks {
        row.add_suffix(&pick_button(state, property, *pick));
    }

    row
}

fn pick_button(state: &AppState, property: &'static str, pick: Pick) -> gtk::Button {
    let (icon_name, tooltip) = match pick {
        Pick::File(_) => ("document-open-symbolic", "Choose a file"),
        Pick::Folder => ("folder-open-symbolic", "Choose a folder"),
    };
    let button = gtk::Button::builder()
        .icon_name(icon_name)
        .tooltip_text(tooltip)
        .css_classes(["flat"])
        .valign(gtk::Align::Center)
        .build();

    button.connect_clicked(glib::clone!(
        #[weak]
        state,
        move |button| {
            let window = button.root().and_downcast::<gtk::Window>();
            let on_chosen = move |chosen: Result<gio::File, glib::Error>| {
                if let Some(path) = chosen.ok().and_then(|file| file.path()) {
                    state.set_property(property, path.display().to_string());
                }
            };
            match pick {
                Pick::File(extensions) => {
                    dialog_for(extensions).open(window.as_ref(), gio::Cancellable::NONE, on_chosen)
                }
                Pick::Folder => gtk::FileDialog::builder()
                    .modal(true)
                    .build()
                    .select_folder(window.as_ref(), gio::Cancellable::NONE, on_chosen),
            }
        }
    ));

    button
}

fn dialog_for(extensions: &[&str]) -> gtk::FileDialog {
    let dialog = gtk::FileDialog::builder().modal(true).build();
    if !extensions.is_empty() {
        let filter = gtk::FileFilter::new();
        for extension in extensions {
            filter.add_suffix(extension);
        }
        dialog.set_default_filter(Some(&filter));
    }
    dialog
}

fn shown(path: &str) -> String {
    if path.is_empty() {
        NOTHING_CHOSEN.to_owned()
    } else {
        path_for_display(&PathBuf::from(path))
    }
}
