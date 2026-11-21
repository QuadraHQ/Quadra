use crate::components::file_row::{ChosenPath, Pick, file_row};
use crate::components::preferences::preference_group;
use crate::components::style::Look;
use crate::pages::settings::bound_rows::choice_row;
use crate::quadra::Message;
use crate::state::{AppState, CursorStyle};
use iced::Element;

const CURSOR_FILES: &[Pick] = &[Pick::File(&["png"]), Pick::Folder];

pub fn cursors_group<'a>(state: &AppState, look: Look) -> Element<'a, Message> {
    let style = state.cursor_style();
    let custom_row = (style == CursorStyle::Custom).then(|| {
        file_row(
            state,
            look,
            ChosenPath::Cursor,
            "Custom cursor",
            CURSOR_FILES,
        )
    });

    preference_group(look)
        .title("Cursors")
        .push(
            choice_row(look, "Cursor style", style, |state, style| {
                state.set_cursor_style(style)
            })
            .subtitle("Replaces the mouse cursors inside the game"),
        )
        .push_maybe(custom_row)
        .into()
}
