use crate::components::file_row::{ChosenPath, Pick, file_row};
use crate::components::preferences::preference_group;
use crate::components::style::Look;
use crate::quadra::Message;
use crate::state::AppState;
use iced::Element;

const FONT_FILES: &[Pick] = &[Pick::File(&["ttf", "otf"])];

pub fn typography_group<'a>(state: &AppState, look: Look) -> Element<'a, Message> {
    preference_group(look)
        .title("Typography")
        .push(file_row(
            state,
            look,
            ChosenPath::Font,
            "Custom font",
            FONT_FILES,
        ))
        .into()
}
