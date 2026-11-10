mod cursors;
mod folder;
mod set_aside;
mod sounds;
mod typography;

use crate::state::AppState;
use adw::prelude::*;
use cursors::cursors_group;
use folder::folder_group;
use set_aside::set_aside_group;
use sounds::sounds_group;
use typography::typography_group;

pub fn mods_section(state: &AppState) -> gtk::Box {
    let section = gtk::Box::new(gtk::Orientation::Vertical, 12);
    section.append(&sounds_group(state));
    section.append(&cursors_group(state));
    section.append(&typography_group(state));
    section.append(&folder_group(state));
    section.append(&set_aside_group());
    section
}
