use super::bound_rows::choice_row;
use super::environment_variables::environment_variables_group;
use super::rosemary_options::{graphics_group, input_group};
use super::rosemary_runtime::rosemary_runtime_group;
use crate::state::{AppState, Runtime};
use adw::prelude::*;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn env_section(state: &AppState) -> gtk::Box {
    // Quadra group
    let check_update_btn = gtk::Button::builder()
        .label("Check for updates")
        .valign(gtk::Align::Center)
        .build();
    check_update_btn.connect_clicked(|_| println!("install clicked"));

    let launcher_info = adw::ActionRow::builder()
        .title("Launcher version")
        .subtitle(format!("v{}", VERSION))
        .build();
    launcher_info.add_suffix(&check_update_btn);

    let force_update_btn = adw::ButtonRow::builder().title("Force update").build();

    let quadra_group = adw::PreferencesGroup::builder().title("Quadra").build();
    quadra_group.add(&launcher_info);
    quadra_group.add(&force_update_btn);

    // Runtime selection
    let runtime_row = choice_row::<Runtime>(state, "runtime", "Run Roblox with");

    let runtime_group = adw::PreferencesGroup::builder()
        .title("Runtime")
        .description("Which compatibility layer the macOS Roblox client runs on.")
        .build();
    runtime_group.add(&runtime_row);

    let section = gtk::Box::new(gtk::Orientation::Vertical, 12);
    section.append(&quadra_group);
    section.append(&runtime_group);
    section.append(&rosemary_runtime_group(state));
    section.append(&graphics_group(state));
    section.append(&input_group(state));
    section.append(&environment_variables_group(state));

    section
}
