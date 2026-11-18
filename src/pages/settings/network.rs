use super::privacy::privacy_group;
use crate::state::AppState;
use adw::prelude::*;

pub fn network_section(state: &AppState) -> gtk::Box {
    let section = gtk::Box::new(gtk::Orientation::Vertical, 12);
    section.append(&privacy_group(state));
    section
}
