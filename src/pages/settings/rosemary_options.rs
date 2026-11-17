use super::bound_rows::{choice_row, choice_row_when, switch_row, switch_row_when};
use super::mangohud::mangohud_row;
use super::{Settings, SettingsMessage};
use crate::components::preferences::{preference_group, preference_row};
use crate::components::style::{self, Look};
use crate::quadra::Message;
use crate::state::{AppState, CommandKey, Renderer, USE_MONITOR_REFRESH_RATE};
use iced::Element;
use iced::widget::text_input;

const REFRESH_RATE_INPUT_WIDTH: f32 = 140.0;

pub fn graphics_group<'a>(
    settings: &'a Settings,
    state: &AppState,
    look: Look,
) -> Element<'a, Message> {
    let renderer = state.renderer();

    preference_group(look)
        .title("Graphics")
        .description("Rosemary options for how Roblox draws to the screen.")
        .push(
            choice_row(look, "Renderer", renderer, |state, renderer| {
                state.set_renderer(renderer)
            })
            .subtitle("The graphics API used for Rosemary. Use OpenGL if you have older hardware."),
        )
        .push(
            choice_row_when(
                look,
                "Vulkan presentation mode",
                state.presentation_mode(),
                |state, mode| state.set_presentation_mode(mode),
                renderer == Renderer::Metal,
            )
            .subtitle("How frames are shown on screen. Immediate turns V-Sync off."),
        )
        .push(
            choice_row_when(
                look,
                "OpenGL swap interval",
                state.opengl_swap_interval(),
                |state, interval| state.set_opengl_swap_interval(interval),
                renderer == Renderer::OpenGl,
            )
            .subtitle("Used instead of the presentation mode with the OpenGL renderer."),
        )
        .push(refresh_rate_row(settings, state, look))
        .push(mangohud_row(state, look))
        .into()
}

pub fn input_group<'a>(state: &AppState, look: Look) -> Element<'a, Message> {
    preference_group(look)
        .title("Input & windows")
        .description("Rosemary options for the keyboard, mouse and window.")
        .push(
            choice_row(look, "Title bar", state.title_bar(), |state, title_bar| {
                state.set_title_bar(title_bar)
            })
            .subtitle("Mac-style shows Roblox's menus inside the title bar."),
        )
        .push(
            switch_row(
                look,
                "Show Roblox's menus",
                state.show_menu_bar(),
                |state, on| state.set_show_menu_bar(on),
            )
            .subtitle(
                "The Roblox, Edit and Window menus. Their shortcuts keep working when hidden.",
            ),
        )
        .push(
            choice_row(look, "Command key", state.command_key(), |state, key| {
                state.set_command_key(key)
            })
            .subtitle("The key Roblox sees as ⌘ Command."),
        )
        .push(
            switch_row_when(
                look,
                "Ctrl for editing shortcuts",
                state.ctrl_for_edit_shortcuts(),
                |state, on| state.set_ctrl_for_edit_shortcuts(on),
                state.command_key() != CommandKey::Ctrl,
            )
            .subtitle("Ctrl+A, C, V, X, Y and Z act as their ⌘ Command versions."),
        )
        .push(
            switch_row(
                look,
                "Plain Ctrl+click",
                state.plain_ctrl_click(),
                |state, on| state.set_plain_ctrl_click(on),
            )
            .subtitle("Treat Ctrl+left click as a left click instead of a right click."),
        )
        .into()
}

fn refresh_rate_row<'a>(
    settings: &'a Settings,
    state: &AppState,
    look: Look,
) -> Element<'a, Message> {
    let shown = settings
        .invalid_refresh_rate
        .clone()
        .unwrap_or_else(|| refresh_rate_as_text(state.refresh_rate()));

    let input = text_input("The monitor's", &shown)
        .font(look.font)
        .size(14)
        .width(REFRESH_RATE_INPUT_WIDTH)
        .padding([6, 10])
        .style(style::input(look, settings.invalid_refresh_rate.is_none()))
        .on_input(|text| Message::Settings(SettingsMessage::RefreshRateChanged(text)));

    preference_row(look, "Refresh rate (Hz)")
        .subtitle("The refresh rate Roblox is told the display has. Default's to the monitor's refresh rate.")
        .suffix(input)
        .into()
}

fn refresh_rate_as_text(rate: u32) -> String {
    if rate == USE_MONITOR_REFRESH_RATE {
        String::new()
    } else {
        rate.to_string()
    }
}

pub fn refresh_rate_from_text(text: &str) -> Option<u32> {
    let text = text.trim();
    if text.is_empty() {
        Some(USE_MONITOR_REFRESH_RATE)
    } else {
        text.parse().ok()
    }
}
