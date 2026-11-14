use super::bound_rows::choice_row;
use super::rosemary_options::{graphics_group, input_group};
use super::rosemary_runtime::rosemary_runtime_group;
use crate::state::{AppState, Runtime};
use adw::prelude::*;

pub fn runtime_section(state: &AppState) -> gtk::Box {
    // Runtime selection
    let runtime_row = choice_row::<Runtime>(state, "runtime", "Run Roblox with");

    let runtime_group = adw::PreferencesGroup::builder()
        .title("Runtime")
        .description("Which compatibility layer the macOS Roblox client runs on.")
        .build();
    runtime_group.add(&runtime_row);

    let section = gtk::Box::new(gtk::Orientation::Vertical, 12);
    section.append(&runtime_group);
    section.append(&rosemary_runtime_group(state));
    section.append(&graphics_group(state));
    section.append(&input_group(state));

    section
}