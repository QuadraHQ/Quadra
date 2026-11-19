use super::account::account_group;
use super::bound_rows::{choice_row, switch_row};
use super::discord_access::ask_for_discord_access_when_turned_on;
use super::runtime::runtime_section;
use super::session::session_group;
use super::support::support_group;
use super::updates::launcher_version_row;
use crate::state::{AppState, LaunchMode, Theme};
use adw::prelude::*;

pub fn general_section(state: &AppState) -> gtk::Box {
    let group = adw::PreferencesGroup::builder().title("Quadra").build();

    let launch_mode_row = choice_row::<LaunchMode>(state, "launch-mode", "When Quadra starts");
    launch_mode_row.set_subtitle("Settings stay available from the system tray");

    let run_in_background_row = switch_row(state, "run-in-background", "Run in the background");
    run_in_background_row.set_subtitle("Keep Quadra in the system tray after its window closes");

    let theme_row = choice_row::<Theme>(state, "theme", "App theme");
    theme_row.set_subtitle("Change the theme to what you like.");

    let advanced_row = switch_row(state, "advanced-settings-visible", "Advanced settings");
    advanced_row.set_subtitle("Only enable if you know what you're doing.");

    group.add(&launcher_version_row(state));
    group.add(&launch_mode_row);
    group.add(&run_in_background_row);
    group.add(&theme_row);
    group.add(&advanced_row);

    let discord_rich_presence_row =
        switch_row(state, "discord-rich-presence", "Show what you're playing");
    discord_rich_presence_row
        .set_subtitle("Show the Roblox game you're in on your Discord profile");
    ask_for_discord_access_when_turned_on(&discord_rich_presence_row);

    let discord_group = adw::PreferencesGroup::builder().title("Discord").build();
    discord_group.add(&discord_rich_presence_row);

    let section = gtk::Box::new(gtk::Orientation::Vertical, 12);
    section.append(&group);
    section.append(&discord_group);
    section.append(&runtime_section(state));

    section.append(&session_group(state));
    section.append(&super::privacy::privacy_group(state));
    section.append(&account_group(state));
    section.append(&support_group(state));

    section
}
