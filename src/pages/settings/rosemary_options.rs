use super::bound_rows::{choice_row, switch_row};
use super::mangohud::mangohud_row;
use crate::state::{
    AppState, CommandKey, PresentationMode, SwapInterval, TitleBar, USE_MONITOR_REFRESH_RATE,
};
use adw::prelude::*;

pub fn graphics_group(state: &AppState) -> adw::PreferencesGroup {
    let opengl_row = switch_row(state, "force-opengl", "Force OpenGL");
    opengl_row.set_subtitle("For machines without a GPU that supports Vulkan");

    let presentation_mode_row =
        choice_row::<PresentationMode>(state, "presentation-mode", "Vulkan presentation mode");
    presentation_mode_row
        .set_subtitle("How frames are shown on screen. Immediate turns V-Sync off.");

    let swap_interval_row =
        choice_row::<SwapInterval>(state, "opengl-swap-interval", "OpenGL swap interval");
    swap_interval_row.set_subtitle("Used instead of the presentation mode when OpenGL is forced.");

    state
        .bind_property("force-opengl", &presentation_mode_row, "sensitive")
        .invert_boolean()
        .sync_create()
        .build();

    state
        .bind_property("force-opengl", &swap_interval_row, "sensitive")
        .sync_create()
        .build();

    let group = adw::PreferencesGroup::builder()
        .title("Graphics")
        .description("Rosemary options for how Roblox draws to the screen.")
        .build();
    group.add(&opengl_row);
    group.add(&presentation_mode_row);
    group.add(&swap_interval_row);
    group.add(&refresh_rate_row(state));
    group.add(&mangohud_row(state));

    group
}

pub fn input_group(state: &AppState) -> adw::PreferencesGroup {
    let title_bar_row = choice_row::<TitleBar>(state, "title-bar", "Title bar");
    title_bar_row.set_subtitle("Mac-style shows Roblox's menus inside the title bar.");

    let command_key_row = choice_row::<CommandKey>(state, "command-key", "Command key");
    command_key_row.set_subtitle("The key Roblox sees as ⌘ Command.");

    let ctrl_for_edit_shortcuts_row = switch_row(
        state,
        "ctrl-for-edit-shortcuts",
        "Ctrl for editing shortcuts",
    );
    ctrl_for_edit_shortcuts_row
        .set_subtitle("Ctrl+A, C, V, X, Y and Z act as their ⌘ Command versions.");

    let plain_ctrl_click_row = switch_row(state, "plain-ctrl-click", "Plain Ctrl+click");
    plain_ctrl_click_row
        .set_subtitle("Treat Ctrl+left click as a left click instead of a right click.");

    state
        .bind_property("command-key", &ctrl_for_edit_shortcuts_row, "sensitive")
        .transform_to(|_, command_key: CommandKey| Some(command_key != CommandKey::Ctrl))
        .sync_create()
        .build();

    let group = adw::PreferencesGroup::builder()
        .title("Input &amp; windows")
        .description("Rosemary options for the keyboard, mouse and window.")
        .build();
    group.add(&title_bar_row);
    group.add(&command_key_row);
    group.add(&ctrl_for_edit_shortcuts_row);
    group.add(&plain_ctrl_click_row);

    group
}

fn refresh_rate_row(state: &AppState) -> adw::ActionRow {
    let refresh_rate_input = gtk::Entry::builder()
        .placeholder_text("The monitor's")
        .valign(gtk::Align::Center)
        .build();

    let row = adw::ActionRow::builder()
        .title("Refresh rate (Hz)")
        .subtitle("The refresh rate Roblox is told the display has. Default's to the monitor's refresh rate.")
        .build();
    row.add_suffix(&refresh_rate_input);

    state
        .bind_property("refresh-rate", &refresh_rate_input, "text")
        .transform_to(|_, rate: u32| Some(refresh_rate_as_text(rate)))
        .transform_from(|_, text: String| refresh_rate_from_text(&text))
        .bidirectional()
        .sync_create()
        .build();

    row
}

fn refresh_rate_as_text(rate: u32) -> String {
    if rate == USE_MONITOR_REFRESH_RATE {
        String::new()
    } else {
        rate.to_string()
    }
}

fn refresh_rate_from_text(text: &str) -> Option<u32> {
    let text = text.trim();
    if text.is_empty() {
        Some(USE_MONITOR_REFRESH_RATE)
    } else {
        text.parse().ok()
    }
}
