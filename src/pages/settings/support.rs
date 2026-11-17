use super::{Settings, SettingsMessage};
use crate::backend;
use crate::components::preferences::{preference_group, preference_row, push_button};
use crate::components::style::{Emphasis, Look};
use crate::quadra::{Message, act};
use crate::state::AppState;
use iced::{Element, Length};

const BYTES_PER_MEGABYTE: u64 = 1_000_000;
const CACHE_SUBTITLE: &str = "Deletes the downloaded interface updates and assets. Your sign-in is kept, and the next launch downloads them again";

pub fn support_group<'a>(
    settings: &'a Settings,
    state: &AppState,
    look: Look,
) -> Element<'a, Message> {
    let is_idle = !state.busy();
    let cache_subtitle = match settings.cleared_cache_bytes.get() {
        Some(freed) => format!("Cleared {} MB", freed / BYTES_PER_MEGABYTE),
        None => CACHE_SUBTITLE.to_owned(),
    };

    preference_group(look)
        .title("Support")
        .push(
            preference_row(look, "Diagnostics")
                .subtitle("Save your settings, hardware details and recent Roblox log for a bug report. Cookies and join tickets are removed")
                .suffix(
                    push_button(look, "Collect", Emphasis::Normal)
                        .on_press_maybe(is_idle.then(|| act(backend::collect_diagnostics_in_background))),
                ),
        )
        .push(
            preference_row(look, "Roblox cache")
                .subtitle(cache_subtitle)
                .suffix(
                    push_button(look, "Clear", Emphasis::Normal)
                        .on_press_maybe(is_idle.then_some(Message::Settings(SettingsMessage::ClearCache))),
                ),
        )
        .push(
            push_button(look, "About Quadra", Emphasis::Flat)
                .on_press(Message::ShowAbout)
                .width(Length::Fill),
        )
        .into()
}
