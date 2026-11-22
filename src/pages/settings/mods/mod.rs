mod cursors;
mod folder;
mod set_aside;
mod sounds;
mod typography;

use super::Settings;
use crate::components::style::Look;
use crate::quadra::Message;
use crate::state::AppState;
use cursors::cursors_group;
use folder::folder_group;
use iced::Element;
use set_aside::set_aside_group;
use sounds::sounds_group;
use typography::typography_group;

pub fn mods_section<'a>(
    settings: &'a Settings,
    state: &AppState,
    look: Look,
) -> Vec<Element<'a, Message>> {
    let mut sections = vec![
        sounds_group(state, look),
        cursors_group(state, look),
        typography_group(state, look),
        folder_group(state, look),
    ];
    sections.extend(set_aside_group(
        &settings.set_aside_mods,
        &settings.mod_problems,
        look,
    ));
    sections
}
