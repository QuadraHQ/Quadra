use super::bound_rows::switch_row;
use crate::components::preferences::preference_group;
use crate::components::style::Look;
use crate::quadra::Message;
use crate::state::AppState;
use iced::Element;

pub fn privacy_group<'a>(state: &AppState, look: Look) -> Element<'a, Message> {
    preference_group(look)
        .title("Privacy")
        .push(
            switch_row(look, "Block Roblox telemetry", state.block_telemetry(), |state, on| {
                state.set_block_telemetry(on)
            })
            .subtitle(
                "Makes Roblox's telemetry and crash-upload hosts unreachable. Applies the next time Roblox starts",
            ),
        )
        .into()
}
