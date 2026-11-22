use super::bound_rows::{SpinRange, spin_row, switch_row};
use crate::components::preferences::preference_group;
use crate::components::style::Look;
use crate::quadra::Message;
use crate::state::AppState;
use iced::Element;

pub fn session_group<'a>(state: &AppState, look: Look) -> Element<'a, Message> {
    let cores = std::thread::available_parallelism().map_or(1, |cores| cores.get() as u32);

    preference_group(look)
        .title("While playing")
        .push(
            switch_row(
                look,
                "Hide Quadra while playing",
                state.hide_launcher_while_playing(),
                |state, on| state.set_hide_launcher_while_playing(on),
            )
            .subtitle("Closes the window once Roblox opens. Quadra quits with the game"),
        )
        .push(switch_row(
            look,
            "Open Quadra after Roblox exits",
            state.open_after_exit(),
            |state, on| state.set_open_after_exit(on),
        ))
        .push(
            switch_row(look, "Show playtime", state.show_playtime(), |state, on| {
                state.set_show_playtime(on)
            })
            .subtitle("Show your total Roblox playtime on the home page"),
        )
        .push(
            switch_row(
                look,
                "Notify of server location",
                state.notify_server_location(),
                |state, on| state.set_notify_server_location(on),
            )
            .subtitle(
                "Looks up where your server is through RoValra and shows a notification. \
                 Only the server's address is sent",
            ),
        )
        .push(
            switch_row(look, "Force RakNet", state.force_raknet(), |state, on| {
                state.set_force_raknet(on)
            })
            .subtitle(
                "Experimental. Patches Roblox to connect to servers over RakNet instead of RbxTransport. \
                 Only works on Roblox versions Quadra knows. Applies the next time Roblox starts",
            ),
        )
        .push(
            spin_row(
                look,
                "Limit CPU cores",
                state.cpu_core_limit(),
                SpinRange {
                    values: 0..=cores,
                    step: 1,
                },
                |state, cores| state.set_cpu_core_limit(cores),
                true,
            )
            .subtitle("Experimental. 0 lets Roblox use every core"),
        )
        .into()
}
