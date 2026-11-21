use super::account::account_group;
use super::appearance::appearance_rows;
use super::bound_rows::{choice_row, switch, switch_row};
use super::privacy::privacy_group;
use super::runtime::runtime_section;
use super::session::session_group;
use super::support::support_group;
use super::updates::launcher_version_row;
use super::{Settings, SettingsMessage};
use crate::components::preferences::{preference_group, preference_row};
use crate::components::style::Look;
use crate::quadra::Message;
use crate::state::AppState;
use iced::Element;

pub fn general_section<'a>(
    settings: &'a Settings,
    state: &AppState,
    look: Look,
) -> Vec<Element<'a, Message>> {
    let quadra_group = preference_group(look)
        .title("Quadra")
        .push(launcher_version_row(state, look))
        .push(
            choice_row(
                look,
                "When Quadra starts",
                state.launch_mode(),
                |state, mode| state.set_launch_mode(mode),
            )
            .subtitle("Settings stay available from the system tray"),
        )
        .push(
            switch_row(
                look,
                "Run in the background",
                state.run_in_background(),
                |state, on| state.set_run_in_background(on),
            )
            .subtitle("Keep Quadra in the system tray after its window closes"),
        );
    let quadra_group = appearance_rows(state, look)
        .into_iter()
        .fold(quadra_group, |group, row| group.push(row))
        .push(
            switch_row(
                look,
                "Use the system title bar",
                state.system_title_bar(),
                |state, on| state.set_system_title_bar(on),
            )
            .subtitle("Let your desktop draw the window's title bar and borders"),
        )
        .push(
            switch_row(
                look,
                "Advanced settings",
                state.advanced_settings_visible(),
                |state, visible| state.set_advanced_settings_visible(visible),
            )
            .subtitle("Only enable if you know what you're doing."),
        );

    let discord_group = preference_group(look).title("Discord").push(
        preference_row(look, "Show what you're playing")
            .subtitle("Show the Roblox game you're in on your Discord profile")
            .suffix(switch(look, state.discord_rich_presence(), true, |on| {
                Message::Settings(SettingsMessage::DiscordPresenceToggled(on))
            })),
    );

    let mut sections = vec![quadra_group.into(), discord_group.into()];
    sections.extend(runtime_section(settings, state, look));
    sections.extend([
        session_group(state, look),
        privacy_group(state, look),
        account_group(state, look),
        support_group(settings, state, look),
    ]);
    sections
}
