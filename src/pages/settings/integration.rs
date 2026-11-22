use super::bound_rows::switch;
use super::{Settings, SettingsMessage};
use crate::backend::{Handler, Shortcut};
use crate::components::preferences::{preference_group, preference_row, push_button};
use crate::components::style::{Emphasis, Look};
use crate::quadra::Message;
use iced::Element;

pub fn integration_section<'a>(settings: &'a Settings, look: Look) -> Vec<Element<'a, Message>> {
    let shortcuts = Shortcut::ALL.map(|shortcut| {
        let entry = shortcut.entry();
        let is_added = settings.added_shortcuts.contains(&shortcut);
        let (label, emphasis) = if is_added {
            ("Remove", Emphasis::Normal)
        } else {
            ("Add", Emphasis::Suggested)
        };
        preference_row(look, entry.name)
            .subtitle(entry.description)
            .suffix(
                push_button(look, label, emphasis)
                    .on_press(Message::Settings(SettingsMessage::ToggleShortcut(shortcut))),
            )
    });

    let handlers = Handler::ALL.map(|handler| {
        let is_enabled = settings.enabled_handlers.contains(&handler);
        preference_row(look, handler.title())
            .subtitle(handler.description())
            .suffix(switch(look, is_enabled, true, move |enabled| {
                Message::Settings(SettingsMessage::ToggleHandler(handler, enabled))
            }))
    });

    vec![
        preference_group(look)
            .title("App menu")
            .description(
                "Adds entries to your app menu. The Roblox and Roblox Studio entries start them \
                 straight away, showing only their progress while they open.",
            )
            .extend(shortcuts)
            .into(),
        preference_group(look)
            .title("Links and files")
            .description("Makes Quadra the app your system uses for these.")
            .extend(handlers)
            .into(),
    ]
}
