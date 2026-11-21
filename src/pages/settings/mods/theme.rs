use crate::components::preferences::preference_group;
use crate::components::style::Look;
use crate::pages::settings::bound_rows::switch_row;
use crate::quadra::Message;
use crate::state::AppState;
use iced::Element;

pub fn theme_group<'a>(state: &AppState, look: Look) -> Element<'a, Message> {
    preference_group(look)
        .title("Theme")
        .push(
            switch_row(
                look,
                "Match Roblox to my theme",
                state.theme_roblox(),
                |state, on| state.set_theme_roblox(on),
            )
            .subtitle(
                "Draws the in-game cursors in your theme's accent colour and uses its font. \
                 A cursor style or font you pick below takes priority",
            ),
        )
        .into()
}
