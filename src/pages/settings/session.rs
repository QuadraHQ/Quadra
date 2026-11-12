use super::bound_rows::switch_row;
use crate::state::AppState;
use adw::prelude::*;

pub fn session_group(state: &AppState) -> adw::PreferencesGroup {
    let hide_row = switch_row(
        state,
        "hide-launcher-while-playing",
        "Hide Quadra while playing",
    );
    hide_row.set_subtitle("Closes the window once Roblox opens. Quadra quits with the game");

    let reopen_row = switch_row(state, "open-after-exit", "Open Quadra after Roblox exits");
    let playtime_row = switch_row(state, "show-playtime", "Show playtime");
    playtime_row.set_subtitle("Show your total Roblox playtime on the home page");

    let location_row = switch_row(state, "notify-server-location", "Notify of server location");
    location_row.set_subtitle(
        "Looks up where your server is through RoValra and shows a notification. \
         Only the server's address is sent",
    );

    let raknet_row = switch_row(state, "force-raknet", "Force RakNet");
    raknet_row.set_subtitle(
        "Experimental. Patches Roblox to connect to servers over RakNet instead of RbxTransport. \
         Only works on Roblox versions Quadra knows. Applies the next time Roblox starts",
    );

    let group = adw::PreferencesGroup::builder()
        .title("While playing")
        .build();
    group.add(&hide_row);
    group.add(&reopen_row);
    group.add(&playtime_row);
    group.add(&location_row);
    group.add(&raknet_row);
    group.add(&cpu_core_limit_row(state));
    group
}

fn cpu_core_limit_row(state: &AppState) -> adw::SpinRow {
    let cores = std::thread::available_parallelism().map_or(1, |cores| cores.get());
    let row = adw::SpinRow::with_range(0.0, cores as f64, 1.0);
    row.set_title("Limit CPU cores");
    row.set_subtitle("Experimental. 0 lets Roblox use every core");

    state
        .bind_property("cpu-core-limit", &row, "value")
        .transform_to(|_, cores: u32| Some(f64::from(cores)))
        .transform_from(|_, cores: f64| Some(cores as u32))
        .bidirectional()
        .sync_create()
        .build();

    row
}
