use crate::components::icon::Symbol;
use crate::components::play_action::PlayAction;
use crate::components::preferences::{symbol_button, with_tooltip};
use crate::components::style::{Emphasis, Look};
use crate::quadra::{Message, act};
use crate::state::{AppState, Client};
use iced::Element;

fn emphasis(action: PlayAction) -> Emphasis {
    match action {
        PlayAction::Install(_) | PlayAction::Play(_) => Emphasis::Suggested,
        PlayAction::Stop => Emphasis::Destructive,
        PlayAction::Opening | PlayAction::Stopping => Emphasis::Normal,
    }
}

fn symbol(action: PlayAction) -> Symbol {
    match action {
        PlayAction::Install(_) => Symbol::Download,
        PlayAction::Play(_) | PlayAction::Opening => Symbol::Play,
        PlayAction::Stop | PlayAction::Stopping => Symbol::Stop,
    }
}

pub fn play_button<'a>(state: &AppState, look: Look, client: Client) -> Element<'a, Message> {
    let action = PlayAction::for_client(state, client);
    let button = symbol_button(look, symbol(action), action.label(), emphasis(action))
        .padding([10, 28])
        .on_press_maybe(
            action
                .is_available(state)
                .then(|| act(move |state| PlayAction::for_client(state, client).perform(state))),
        );

    match action.explanation_when_unavailable(state) {
        Some(explanation) => with_tooltip(look, button, explanation),
        None => button.into(),
    }
}
