use super::environment_variables::environment_variables_group;
use super::launch_wrapper::launch_wrapper_group;
use crate::state::AppState;
use adw::prelude::*;

pub fn advanced_section(state: &AppState) -> gtk::Box {
    let section = gtk::Box::new(gtk::Orientation::Vertical, 12);
    section.append(&launch_wrapper_group(state));
    section.append(&environment_variables_group(state));
    section
}