use crate::components::command_dialog::command_dialog;
use crate::components::preferences::{preference_row, push_button};
use crate::components::style::{Emphasis, Look};
use crate::quadra::Message;
use iced::Element;

pub fn butter_access_row<'a>(command: Option<&str>, look: Look) -> Option<Element<'a, Message>> {
    let command = command?;
    let dialog = command_dialog(
        "Let Quadra see Butter runners",
        "Run this in a terminal to let Quadra read the runners Butter installed, then restart Quadra:",
        command,
    );

    Some(
        preference_row(look, "Looking for Butter runners?")
            .subtitle("Quadra runs as a Flatpak, so it can only list the runners Butter installed once you allow it.")
            .suffix(push_button(look, "Show how", Emphasis::Normal).on_press(Message::Alert(dialog)))
            .into(),
    )
}
