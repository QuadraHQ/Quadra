use crate::backend;
use crate::components::preferences::{preference_group, preference_row, push_button};
use crate::components::style::{Emphasis, Look};
use crate::pages::settings::bound_rows::switch_row;
use crate::quadra::{Message, act};
use crate::state::AppState;
use crate::utils::open_with_default_app;
use iced::Element;

pub fn folder_group<'a>(state: &AppState, look: Look) -> Element<'a, Message> {
    preference_group(look)
        .title("User modifications")
        .push(
            switch_row(
                look,
                "Use the modifications folder",
                state.custom_mods(),
                |state, on| state.set_custom_mods(on),
            )
            .subtitle("Overlays its files onto the Roblox client when it starts"),
        )
        .push(
            preference_row(look, "Modifications folder")
                .subtitle("Drop your own textures, sounds and models here")
                .suffix(push_button(look, "Open", Emphasis::Normal).on_press(act(open_folder))),
        )
        .into()
}

fn open_folder(state: &AppState) {
    match backend::mods_folder() {
        Ok(directory) => open_with_default_app(directory),
        Err(error) => {
            state.report_error(&format!("Couldn't open the modifications folder: {error}"))
        }
    }
}
