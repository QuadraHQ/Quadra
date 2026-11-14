use super::account::account_group;
use super::bound_rows::{choice_row, switch_row};
use super::discord_access::ask_for_discord_access_when_turned_on;
use super::session::session_group;
use super::support::support_group;
use super::updates::launcher_version_row;
use crate::state::{AppState, LaunchMode, Theme};
use adw::prelude::*;

pub fn general_section(state: &AppState) -> gtk::Box {
    // Quadra group
    let launch_mode_row = choice_row::<LaunchMode>(state, "launch-mode", "When Quadra starts");
    launch_mode_row.set_subtitle("Settings stay available from the system tray");

    let run_in_background_row = switch_row(state, "run-in-background", "Run in the background");
    run_in_background_row.set_subtitle("Keep Quadra in the system tray after its window closes");

    let theme_row = choice_row::<Theme>(state, "theme", "App theme");
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
    ask_for_discord_access_when_turned_on(&discord_rich_presence_row);

    let discord_group = adw::PreferencesGroup::builder().title("Discord").build();
    discord_group.add(&discord_rich_presence_row);

    let section = gtk::Box::new(gtk::Orientation::Vertical, 12);
    section.append(&quadra_group);
    section.append(&discord_group);
    section.append(&session_group(state));
    section.append(&super::privacy::privacy_group(state));
    section.append(&account_group(state));
    section.append(&support_group(state));

    section
}