use crate::backend;
use crate::pages::settings::bound_rows::switch_row;
use crate::state::AppState;
use adw::prelude::*;
use gtk::{gio, glib};

pub fn folder_group(state: &AppState) -> adw::PreferencesGroup {
    let enabled_row = switch_row(state, "custom-mods", "Use the modifications folder");
    enabled_row.set_subtitle("Overlays its files onto the Roblox client when it starts");

    let open_button = gtk::Button::builder()
        .label("Open")
        .valign(gtk::Align::Center)
        .build();
    open_button.connect_clicked(glib::clone!(
        #[weak]
        state,
        move |_| open_folder(&state)
    ));

    let open_row = adw::ActionRow::builder()
        .title("Modifications folder")
        .subtitle("Drop your own textures, sounds and models here")
        .build();
    open_row.add_suffix(&open_button);

    let group = adw::PreferencesGroup::builder()
        .title("User modifications")
        .build();
    group.add(&enabled_row);
    group.add(&open_row);
    group
}

fn open_folder(state: &AppState) {
    match backend::mods_folder() {
        Ok(directory) => {
            gtk::FileLauncher::new(Some(&gio::File::for_path(directory))).launch(
                gtk::Window::NONE,
                gio::Cancellable::NONE,
                |_| {},
            );
        }
        Err(error) => {
            state.report_error(&format!("Couldn't open the modifications folder: {error}"))
        }
    }
}
