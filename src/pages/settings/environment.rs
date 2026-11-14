use super::account::account_group;
use super::bound_rows::{choice_row, switch_row};
use super::environment_variables::environment_variables_group;
use super::launch_wrapper::launch_wrapper_group;
use super::privacy::privacy_group;
use super::rosemary_options::{graphics_group, input_group};
use super::rosemary_runtime::rosemary_runtime_group;
use super::session::session_group;
use super::support::support_group;
use super::updates::launcher_version_row;
use crate::state::{AppState, LaunchMode, Runtime, Theme};
use adw::prelude::*;

pub fn env_section(state: &AppState) -> gtk::Box {
    // Quadra group
    let launch_mode_row = choice_row::<LaunchMode>(state, "launch-mode", "When Quadra starts");
    launch_mode_row.set_subtitle("Settings stay available from the system tray");

    let run_in_background_row = switch_row(state, "run-in-background", "Run in the background");
    run_in_background_row.set_subtitle("Keep Quadra in the system tray after its window closes");

    let theme_row = choice_row::<Theme>(state, "theme", "The app theme.");
    theme_row.set_subtitle("Change the theme to what you like.");

    let quadra_group = adw::PreferencesGroup::builder().title("Quadra").build();
    quadra_group.add(&launcher_version_row(state));
    quadra_group.add(&launch_mode_row);
    quadra_group.add(&run_in_background_row);
    quadra_group.add(&theme_row);

    let discord_rich_presence_row =
        switch_row(state, "discord-rich-presence", "Show what you're playing");
    discord_rich_presence_row
        .set_subtitle("Show the Roblox game you're in on your Discord profile");

    let discord_group = adw::PreferencesGroup::builder().title("Discord").build();
    discord_group.add(&discord_rich_presence_row);

    // Runtime selection
    let runtime_row = choice_row::<Runtime>(state, "runtime", "Run Roblox with");

    let runtime_group = adw::PreferencesGroup::builder()
        .title("Runtime")
        .description("Which compatibility layer the macOS Roblox client runs on.")
        .build();
    runtime_group.add(&runtime_row);

    let section = gtk::Box::new(gtk::Orientation::Vertical, 12);
    section.append(&quadra_group);
    section.append(&discord_group);
    section.append(&session_group(state));
    section.append(&privacy_group(state));
    section.append(&runtime_group);
    section.append(&rosemary_runtime_group(state));
    section.append(&graphics_group(state));
    section.append(&input_group(state));
    section.append(&launch_wrapper_group(state));
    section.append(&environment_variables_group(state));
    section.append(&account_group(state));
    section.append(&support_group(state));

    section
}
