use super::fast_flags::fast_flags_section;
use super::mangohud::mangohud_row;
use super::rosemary_options::refresh_rate_row;
use crate::state::AppState;
use adw::prelude::*;

fn performance_group(state: &AppState) -> adw::PreferencesGroup {
    let mangohud_row = mangohud_row(state);
    mangohud_row.set_subtitle("Show FPS, frame times, and GPU stats in-game");

    let refresh_rate_row = refresh_rate_row(state);
    refresh_rate_row.set_subtitle("Override the refresh rate Roblox detects. Default: monitor's refresh rate.");

    let group = adw::PreferencesGroup::builder()
        .title("Performance")
        .description("In-game performance monitoring and display settings.")
        .build();
    group.add(&mangohud_row);
    group.add(&refresh_rate_row);

    group
}

pub fn performance_section(state: &AppState) -> gtk::Box {
    let section = gtk::Box::new(gtk::Orientation::Vertical, 12);
    section.append(&performance_group(state));
    section.append(&fast_flags_section(state));
    section
}