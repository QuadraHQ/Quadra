use super::account::account_group;
use super::bound_rows::{choice_row, switch, switch_row};
use super::environment_variables::environment_variables_group;
use super::launch_wrapper::launch_wrapper_group;
use super::privacy::privacy_group;
use super::rosemary_options::{graphics_group, input_group};
use super::rosemary_runtime::rosemary_runtime_group;
use super::session::session_group;
use super::support::support_group;
use super::updates::launcher_version_row;
use super::{Settings, SettingsMessage};
use crate::components::preferences::{preference_group, preference_row};
use crate::components::style::Look;
use crate::quadra::Message;
use crate::state::AppState;
use iced::Element;

pub fn env_section<'a>(
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
        )
        .push(
            choice_row(look, "The app theme.", state.theme(), |state, theme| {
                state.set_theme(theme)
            })
            .subtitle("Change the theme to what you like."),
        );

    let discord_group = preference_group(look).title("Discord").push(
        preference_row(look, "Show what you're playing")
            .subtitle("Show the Roblox game you're in on your Discord profile")
            .suffix(switch(look, state.discord_rich_presence(), true, |on| {
                Message::Settings(SettingsMessage::DiscordPresenceToggled(on))
            })),
    );

    let runtime_group = preference_group(look)
        .title("Runtime")
        .description("Which compatibility layer the macOS Roblox client runs on.")
        .push(choice_row(
            look,
            "Run Roblox with",
            state.runtime(),
            |state, runtime| state.set_runtime(runtime),
        ));

    vec![
        quadra_group.into(),
        discord_group.into(),
        session_group(state, look),
        privacy_group(state, look),
        runtime_group.into(),
        rosemary_runtime_group(settings, state, look),
        graphics_group(settings, state, look),
        input_group(state, look),
        launch_wrapper_group(settings, state, look),
        environment_variables_group(&settings.variables, look),
        account_group(state, look),
        support_group(settings, state, look),
    ]
}
