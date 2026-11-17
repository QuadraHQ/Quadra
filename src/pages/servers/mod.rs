mod history_group;
mod join_by_link_row;
mod place_link;
mod region_row;
mod time_since;

use crate::backend;
use crate::components::play_action::PlayAction;
use crate::components::preferences::{page, preference_group};
use crate::components::style::Look;
use crate::quadra::Message;
use crate::state::{AppState, Client};
use history_group::history_groups;
use iced::Element;
use join_by_link_row::join_by_link_row;
use region_row::preferred_region_row;
use std::collections::HashSet;

#[derive(Default)]
pub struct Servers {
    link: String,
    link_is_invalid: bool,
    expanded_games: HashSet<u64>,
    regions_requested: bool,
}

#[derive(Debug, Clone)]
pub enum ServersMessage {
    LinkChanged(String),
    JoinByLink,
    ToggleGame(u64),
}

impl Servers {
    pub fn shown(&mut self, state: &AppState) {
        if !self.regions_requested {
            self.regions_requested = true;
            backend::load_regions(state);
        }
    }

    pub fn update(&mut self, state: &AppState, message: ServersMessage) {
        match message {
            ServersMessage::LinkChanged(link) => {
                self.link = link;
                self.link_is_invalid = false;
            }
            ServersMessage::JoinByLink => match place_link::place_id_in(&self.link) {
                Some(place_id) => {
                    self.link.clear();
                    self.link_is_invalid = false;
                    backend::join_game(state, place_id);
                }
                None => self.link_is_invalid = true,
            },
            ServersMessage::ToggleGame(place_id) => {
                if !self.expanded_games.remove(&place_id) {
                    self.expanded_games.insert(place_id);
                }
            }
        }
    }

    pub fn view<'a>(&'a self, state: &AppState, look: Look) -> Element<'a, Message> {
        let can_join = can_join(state);
        let joining_group = preference_group(look)
            .title("Joining")
            .description(
                "With a preferred region, games started from this page join a server there, \
                 picked from RoValra's server list (apis.rovalra.com). RoValra sees the game and \
                 the region, not your account.",
            )
            .push(preferred_region_row(state, look))
            .push(join_by_link_row(
                &self.link,
                self.link_is_invalid,
                can_join,
                look,
            ));

        let mut sections = vec![joining_group.into()];
        sections.extend(history_groups(state, look, &self.expanded_games, can_join));
        page(sections)
    }
}

fn can_join(state: &AppState) -> bool {
    let action = PlayAction::for_client(state, Client::Player);
    action == PlayAction::Play(Client::Player) && action.is_available(state)
}
