use super::ServersMessage;
use super::time_since::time_since;
use crate::backend;
use crate::components::icon::{self, Symbol};
use crate::components::preferences::{preference_group, preference_row, push_button};
use crate::components::style::{self, Emphasis, Look};
use crate::quadra::{Message, act};
use crate::state::{AppState, JoinedServer, PlayedGame};
use iced::widget::{Column, button, container, rule};
use iced::{Element, Length};
use std::collections::HashSet;

const UNKNOWN_LOCATION: &str = "Unknown location";

pub fn history_groups<'a>(
    state: &AppState,
    look: Look,
    expanded_games: &HashSet<u64>,
    can_join: bool,
) -> Vec<Element<'a, Message>> {
    let history = state.server_history();
    let history_group = preference_group(look).title("Server history").description(
        "Servers you joined with Quadra. Rejoining asks Roblox for the same server, \
         and Roblox picks a new one if it has closed.",
    );

    if history.0.is_empty() {
        let nothing_played =
            preference_row(look, "No servers yet").subtitle("Games you join will show up here.");
        return vec![history_group.push(nothing_played).into()];
    }

    let games = history.0.iter().map(|game| {
        game_row(
            game,
            expanded_games.contains(&game.place_id),
            can_join,
            look,
        )
    });
    let forget_button = push_button(look, "Clear history", Emphasis::Destructive)
        .on_press(act(backend::forget_server_history))
        .width(Length::Fill);

    vec![history_group.extend(games).into(), forget_button.into()]
}

fn game_row<'a>(
    game: &PlayedGame,
    is_expanded: bool,
    can_join: bool,
    look: Look,
) -> Element<'a, Message> {
    let title = if game.name.is_empty() {
        format!("Place {}", game.place_id)
    } else {
        game.name.clone()
    };
    let place_id = game.place_id;

    let play_button = push_button(look, "Play", Emphasis::Suggested)
        .on_press_maybe(can_join.then(|| act(move |state| backend::join_game(state, place_id))));
    let expander = button(icon::symbol(
        if is_expanded {
            Symbol::Collapse
        } else {
            Symbol::Expand
        },
        look,
    ))
    .padding(7)
    .style(style::emphasized(look, Emphasis::Flat))
    .on_press(Message::Servers(ServersMessage::ToggleGame(place_id)));

    let header = preference_row(look, title)
        .subtitle(game.last_played_at().map(time_since).unwrap_or_default())
        .suffix(play_button)
        .suffix(expander);

    let mut rows = Column::new().push(header);
    if is_expanded {
        for server in &game.servers {
            rows = rows
                .push(rule::horizontal(1).style(style::separator(look)))
                .push(server_row(place_id, server, can_join, look));
        }
    }
    rows.into()
}

fn server_row<'a>(
    place_id: u64,
    server: &JoinedServer,
    can_join: bool,
    look: Look,
) -> Element<'a, Message> {
    let title = server
        .location
        .as_deref()
        .or(server.address.as_deref())
        .unwrap_or(UNKNOWN_LOCATION)
        .to_owned();
    let job_id = server.job_id.clone();

    let rejoin_button = push_button(look, "Rejoin", Emphasis::Normal).on_press_maybe(
        can_join.then(|| act(move |state| backend::rejoin_server(state, place_id, &job_id))),
    );

    container(
        preference_row(look, title)
            .subtitle(time_since(server.joined_at))
            .suffix(rejoin_button),
    )
    .padding(iced::padding::left(16))
    .into()
}
