use crate::components::preferences::{preference_group, preference_row, push_button};
use crate::components::style::{Emphasis, Look};
use crate::pages::settings::SettingsMessage;
use crate::quadra::Message;
use iced::Element;
use std::collections::HashMap;

pub fn set_aside_group<'a>(
    units: &'a [String],
    problems: &'a HashMap<String, String>,
    look: Look,
) -> Option<Element<'a, Message>> {
    if units.is_empty() {
        return None;
    }

    let rows =
        units.iter().map(|unit| {
            preference_row(look, unit.as_str())
                .subtitle_maybe(problems.get(unit).map(String::as_str))
                .suffix(push_button(look, "Re-enable", Emphasis::Normal).on_press(
                    Message::Settings(SettingsMessage::ReenableMod(unit.clone())),
                ))
        });

    Some(
        preference_group(look)
            .title("Set aside by the crash guard")
            .description(
                "Roblox crashed soon after these were added, so they are no longer applied.",
            )
            .extend(rows)
            .into(),
    )
}
