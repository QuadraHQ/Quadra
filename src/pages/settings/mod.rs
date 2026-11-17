mod account;
mod bound_rows;
mod butter_access;
mod custom_flags;
mod discord_access;
mod environment;
mod environment_variables;
mod fast_flag_allowlist;
mod fast_flags;
mod integration;
mod launch_wrapper;
mod mangohud;
mod mods;
mod privacy;
mod rosemary_options;
mod rosemary_runtime;
mod session;
mod support;
mod updates;
mod vr;

use crate::backend::{self, Handler, Shortcut};
use crate::components::file_dialog;
use crate::components::preferences::page;
use crate::components::style::{self, Look};
use crate::quadra::Message;
use crate::state::{AppState, Client, QUADRA_PREFIX};
use crate::utils::is_running_in_flatpak;
use bound_rows::keep_named_choice_valid;
use custom_flags::CustomFlags;
use environment_variables::EnvironmentVariableRows;
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Element, Length, Task};
use std::cell::Cell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

pub use custom_flags::{EXPORT_FILE_NAME, export_dialog, import_flags, save_exported};

pub const SCROLLER: &str = "settings-scroller";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Environment,
    FastFlags,
    Mods,
    Vr,
    Integration,
}

impl Tab {
    fn title(self) -> &'static str {
        match self {
            Self::Environment => "Environment",
            Self::FastFlags => "FastFlags",
            Self::Mods => "Mods",
            Self::Vr => "VR",
            Self::Integration => "Integration",
        }
    }

    fn is_shown(self, state: &AppState) -> bool {
        match self {
            Self::Environment => true,
            Self::FastFlags | Self::Mods | Self::Vr => state.selected_client() == Client::Player,
            Self::Integration => !is_running_in_flatpak(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum SettingsMessage {
    ShowTab(Tab),
    DiscordPresenceToggled(bool),
    MangoHudToggled(bool),
    LaunchWrapperChanged(String),
    RefreshRateChanged(String),
    NumberFlagChanged(&'static str, String),
    CustomFlag(custom_flags::CustomFlagMessage),
    EnvironmentVariable(environment_variables::VariableMessage),
    PickRunner,
    RunnerPicked(Option<PathBuf>),
    ClearCache,
    ReenableMod(String),
    ToggleShortcut(Shortcut),
    ToggleHandler(Handler, bool),
}

pub struct Settings {
    tab: Tab,
    custom_flags: CustomFlags,
    variables: EnvironmentVariableRows,
    invalid_numbers: HashMap<&'static str, String>,
    invalid_refresh_rate: Option<String>,
    launch_wrapper_problem: Option<String>,
    cleared_cache_bytes: Rc<Cell<Option<u64>>>,
    set_aside_mods: Vec<String>,
    mod_problems: HashMap<String, String>,
    added_shortcuts: Vec<Shortcut>,
    enabled_handlers: Vec<Handler>,
    butter_access_command: Option<String>,
}

impl Settings {
    pub fn new(state: &AppState) -> Self {
        Self {
            tab: Tab::default(),
            custom_flags: CustomFlags::from_state(state),
            variables: EnvironmentVariableRows::from_state(state),
            invalid_numbers: HashMap::new(),
            invalid_refresh_rate: None,
            launch_wrapper_problem: backend::launch_wrapper_problem(&state.launch_wrapper()),
            cleared_cache_bytes: Rc::default(),
            set_aside_mods: Vec::new(),
            mod_problems: HashMap::new(),
            added_shortcuts: Vec::new(),
            enabled_handlers: Vec::new(),
            butter_access_command: backend::missing_butter_access(),
        }
    }

    pub fn reload_custom_flags(&mut self, state: &AppState) {
        self.custom_flags = CustomFlags::from_state(state);
    }

    fn shown_tab(&self, state: &AppState) -> Tab {
        if self.tab.is_shown(state) {
            self.tab
        } else {
            Tab::Environment
        }
    }

    pub fn update(&mut self, state: &AppState, message: SettingsMessage) -> Task<Message> {
        match message {
            SettingsMessage::ShowTab(tab) => {
                self.tab = tab;
                match tab {
                    Tab::Mods => self.refresh_set_aside_mods(),
                    Tab::Integration => self.refresh_integration(),
                    Tab::Environment | Tab::FastFlags | Tab::Vr => {}
                }
            }
            SettingsMessage::DiscordPresenceToggled(on) => {
                state.set_discord_rich_presence(on);
                if let Some(dialog) = on.then(discord_access::dialog_if_needed).flatten() {
                    return Task::done(Message::Alert(dialog));
                }
            }
            SettingsMessage::MangoHudToggled(on) => {
                state.set_mangohud(on);
                if on && let Some(dialog) = mangohud::dialog_if_needed() {
                    return Task::done(Message::Alert(dialog));
                }
            }
            SettingsMessage::LaunchWrapperChanged(wrapper) => {
                self.launch_wrapper_problem = backend::launch_wrapper_problem(&wrapper);
                state.set_launch_wrapper(wrapper);
            }
            SettingsMessage::RefreshRateChanged(text) => {
                match rosemary_options::refresh_rate_from_text(&text) {
                    Some(rate) => {
                        self.invalid_refresh_rate = None;
                        state.set_refresh_rate(rate);
                    }
                    None => self.invalid_refresh_rate = Some(text),
                }
            }
            SettingsMessage::NumberFlagChanged(name, text) => {
                fast_flags::number_changed(state, &mut self.invalid_numbers, name, text);
            }
            SettingsMessage::CustomFlag(message) => {
                return self.custom_flags.update(state, message);
            }
            SettingsMessage::EnvironmentVariable(message) => {
                return self.variables.update(state, message);
            }
            SettingsMessage::PickRunner => {
                return file_dialog::pick_file("Choose a Rosemary runner", &[])
                    .map(|chosen| Message::Settings(SettingsMessage::RunnerPicked(chosen)));
            }
            SettingsMessage::RunnerPicked(chosen) => {
                if let Some(executable) = chosen {
                    backend::use_custom_runner(state, &executable);
                }
            }
            SettingsMessage::ClearCache => {
                let cleared = self.cleared_cache_bytes.clone();
                backend::clear_roblox_cache(state, move |freed| cleared.set(Some(freed)));
            }
            SettingsMessage::ReenableMod(unit) => {
                match backend::enable_mod(&unit) {
                    Ok(()) => {
                        self.mod_problems.remove(&unit);
                    }
                    Err(error) => {
                        let problem = format!("Couldn't re-enable {unit}: {error}");
                        self.mod_problems.insert(unit, problem);
                    }
                }
                self.refresh_set_aside_mods();
            }
            SettingsMessage::ToggleShortcut(shortcut) => {
                if backend::has_shortcut(shortcut) {
                    backend::remove_shortcut(state, shortcut);
                } else {
                    backend::add_shortcut(state, shortcut);
                }
                self.refresh_integration();
            }
            SettingsMessage::ToggleHandler(handler, enabled) => {
                if enabled != backend::handler_is_enabled(handler) {
                    backend::set_handler_enabled(state, handler, enabled);
                }
                self.refresh_integration();
            }
        }
        Task::none()
    }

    fn refresh_set_aside_mods(&mut self) {
        self.set_aside_mods = backend::disabled_mods();
    }

    fn refresh_integration(&mut self) {
        self.added_shortcuts = Shortcut::ALL
            .into_iter()
            .filter(|&shortcut| backend::has_shortcut(shortcut))
            .collect();
        self.enabled_handlers = Handler::ALL
            .into_iter()
            .filter(|&handler| backend::handler_is_enabled(handler))
            .collect();
    }

    pub fn view<'a>(&'a self, state: &AppState, look: Look) -> Element<'a, Message> {
        let shown_tab = self.shown_tab(state);
        let tabs = [
            Tab::Environment,
            Tab::FastFlags,
            Tab::Mods,
            Tab::Vr,
            Tab::Integration,
        ]
        .into_iter()
        .filter(|tab| tab.is_shown(state))
        .map(|tab| {
            button(text(tab.title()).font(look.font).size(13))
                .padding([5, 14])
                .style(style::tab(look, tab == shown_tab))
                .on_press(Message::Settings(SettingsMessage::ShowTab(tab)))
                .into()
        });
        let switcher = container(
            container(row(tabs).spacing(4))
                .padding(3)
                .style(style::card(look)),
        )
        .center_x(Length::Fill)
        .padding(iced::padding::top(12));

        let sections = match shown_tab {
            Tab::Environment => environment::env_section(self, state, look),
            Tab::FastFlags => fast_flags::fast_flags_section(self, state, look),
            Tab::Mods => mods::mods_section(self, state, look),
            Tab::Vr => vr::vr_section(state, look),
            Tab::Integration => integration::integration_section(self, look),
        };

        column![
            switcher,
            scrollable(page(sections))
                .id(SCROLLER)
                .height(Length::Fill)
                .style(style::scrollbar(look))
        ]
        .into()
    }
}

pub fn keep_choices_valid(state: &AppState) {
    keep_named_choice_valid(
        state,
        "available-runners",
        AppState::available_runners,
        AppState::runner,
        |state, runner| state.set_runner(runner),
    );
    keep_named_choice_valid(
        state,
        "available-prefixes",
        AppState::available_prefixes,
        AppState::prefix,
        |state, prefix| state.set_prefix(prefix),
    );
}

fn is_quadra_prefix(state: &AppState) -> bool {
    state.prefix() == QUADRA_PREFIX
}
