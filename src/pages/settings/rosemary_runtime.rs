use super::bound_rows::{ChoiceAction, named_choice_row};
use super::butter_access::butter_access_row;
use super::{Settings, SettingsMessage, is_quadra_prefix};
use crate::backend::{self, QUADRA_ROSEMARY_VERSION};
use crate::components::icon::{self, Symbol};
use crate::components::preferences::{
    PreferenceRow, preference_group, preference_row, push_button,
};
use crate::components::style::{Emphasis, Look};
use crate::quadra::{Message, act};
use crate::state::{AppState, QUADRA_RUNNER};
use iced::Element;

pub fn rosemary_runtime_group<'a>(
    settings: &'a Settings,
    state: &AppState,
    look: Look,
) -> Element<'a, Message> {
    let runner_row = named_choice_row(
        look,
        "Runner",
        state.available_runners(),
        state.runner(),
        |state, runner| state.set_runner(runner),
        Some(ChoiceAction {
            label: "Choose a runner file…",
            message: Message::Settings(SettingsMessage::PickRunner),
        }),
    )
    .subtitle(runner_description(state));

    let prefix_row = named_choice_row(
        look,
        "Prefix",
        state.available_prefixes(),
        state.prefix(),
        |state, prefix| state.set_prefix(prefix),
        None,
    )
    .subtitle("The Darwin filesystem Roblox runs in.");

    preference_group(look)
        .title("Rosemary")
        .description("Which Rosemary runs Roblox, and where.")
        .push_maybe(rosemary_install_row(state, look))
        .push_maybe(rosemary_update_row(state, look))
        .push(runner_row)
        .push_maybe(butter_access_row(
            settings.butter_access_command.as_deref(),
            look,
        ))
        .push(prefix_row)
        .push(prefix_status_row(state, look))
        .into()
}

#[derive(Clone, Copy)]
enum PrefixAction {
    InstallDarwinBase,
    UpdateDarwinBase,
    RepairPrefix,
}

impl PrefixAction {
    fn for_state(state: &AppState) -> Self {
        if !state.darwin_base_installed() {
            Self::InstallDarwinBase
        } else if !state.newer_darwin_base().is_empty() {
            Self::UpdateDarwinBase
        } else {
            Self::RepairPrefix
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::InstallDarwinBase => "Install",
            Self::UpdateDarwinBase => "Update",
            Self::RepairPrefix => "Repair",
        }
    }

    fn perform(self, state: &AppState) {
        match self {
            Self::InstallDarwinBase => backend::install_darwin_base(state),
            Self::UpdateDarwinBase => backend::update_darwin_base(state),
            Self::RepairPrefix => backend::repair_prefix(state),
        }
    }
}

fn prefix_status_row<'a>(state: &AppState, look: Look) -> PreferenceRow<'a> {
    let action = PrefixAction::for_state(state);
    let is_quadra_prefix = is_quadra_prefix(state);
    let (subtitle, symbol, color) = match action {
        PrefixAction::InstallDarwinBase => (
            "Darwin base not installed".to_owned(),
            Symbol::Warning,
            look.warning,
        ),
        PrefixAction::UpdateDarwinBase => (
            format!(
                "Darwin base {} is available, it installs when you play",
                state.newer_darwin_base()
            ),
            Symbol::Update,
            look.success,
        ),
        PrefixAction::RepairPrefix => (
            "Darwin base installed".to_owned(),
            Symbol::Check,
            look.success,
        ),
    };
    let can_act = is_quadra_prefix && state.rosemary_installed() && !state.busy();

    preference_row(look, "Prefix status")
        .subtitle(subtitle)
        .prefix(icon::tinted(symbol, color))
        .suffix(
            push_button(look, action.label(), Emphasis::Normal)
                .on_press_maybe(can_act.then(|| act(move |state| action.perform(state)))),
        )
        .enabled(is_quadra_prefix)
}

fn rosemary_install_row<'a>(state: &AppState, look: Look) -> Option<PreferenceRow<'a>> {
    let is_shown = quadra_rosemary_is_selected_but_missing(state) && !state.first_launch();
    is_shown.then(|| {
        preference_row(look, "Rosemary isn't installed")
            .subtitle(format!(
                "Install Quadra's Rosemary {QUADRA_ROSEMARY_VERSION} to run Roblox."
            ))
            .suffix(
                push_button(look, "Install", Emphasis::Suggested)
                    .on_press_maybe((!state.busy()).then(|| act(backend::install_rosemary))),
            )
    })
}

fn rosemary_update_row<'a>(state: &AppState, look: Look) -> Option<PreferenceRow<'a>> {
    let newer_version = state.newer_quadra_rosemary();
    let quadra_rosemary_is_in_use = state.runner() == QUADRA_RUNNER && state.rosemary_installed();
    let is_shown = quadra_rosemary_is_in_use && !newer_version.is_empty();
    let can_update = !state.busy() && !state.running();

    is_shown.then(|| {
        preference_row(look, "A newer Rosemary is available")
            .subtitle(format!(
                "Rosemary {newer_version} installs when you play, or update now."
            ))
            .suffix(
                push_button(look, "Update", Emphasis::Suggested)
                    .on_press_maybe(can_update.then(|| act(backend::update_quadra_rosemary))),
            )
    })
}

fn quadra_rosemary_is_selected_but_missing(state: &AppState) -> bool {
    state.runner() == QUADRA_RUNNER && !state.rosemary_installed()
}

fn runner_description(state: &AppState) -> String {
    if quadra_rosemary_is_selected_but_missing(state) {
        format!("Quadra's Rosemary {QUADRA_ROSEMARY_VERSION} will be installed when you play.")
    } else {
        "The Rosemary build that runs Roblox.".to_owned()
    }
}
