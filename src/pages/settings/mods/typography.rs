use crate::components::file_row::{ChosenPath, Pick, file_row};
use crate::components::preferences::preference_group;
use crate::components::style::Look;
use crate::pages::settings::bound_rows::choice_row;
use crate::quadra::Message;
use crate::state::{AppState, FontStyle};
use iced::Element;

const FONT_FILES: &[Pick] = &[Pick::File(&["ttf", "otf"])];

pub fn typography_group<'a>(state: &AppState, look: Look) -> Element<'a, Message> {
    let style = state.font_style();
    let custom_row = (style == FontStyle::Custom)
        .then(|| file_row(state, look, ChosenPath::Font, "Custom font", FONT_FILES));

    preference_group(look)
        .title("Typography")
        .push(
            choice_row(look, "Font", style, |state, style| {
                state.set_font_style(style)
            })
            .subtitle("Replaces the text font inside the game"),
        )
        .push_maybe(custom_row)
        .into()
}
