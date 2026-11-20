mod choice;
mod client;
mod environment_variables;
mod fast_flags;
mod launch_mode;
mod mods;
mod regions;
mod rosemary_options;
mod server_history;

pub use choice::Choice;
pub use client::Client;
pub use environment_variables::{EnvironmentVariable, EnvironmentVariables};
pub use fast_flags::{FastFlagValue, FastFlags};
pub use launch_mode::LaunchMode;
pub use mods::{CursorStyle, DeathSound};
pub use regions::{Region, Regions, country_and_city_of};
pub use rosemary_options::{CommandKey, PresentationMode, Renderer, SwapInterval, TitleBar};
pub use server_history::{JoinedServer, PlayedGame, ServerHistory};

use std::cell::RefCell;
use std::error::Error;
use std::fs;
use std::io::ErrorKind;
use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::main_loop::{self, Source};
use crate::utils::config_directory;
use serde::{Deserialize, Serialize};

pub const USE_MONITOR_REFRESH_RATE: u32 = 0;
pub const DEFAULT_VR_RENDER_SCALE: u32 = 70;
pub const VR_RENDER_SCALE_RANGE: RangeInclusive<u32> = 40..=150;
pub const QUADRA_RUNNER: &str = "Quadra";
pub const QUADRA_PREFIX: &str = "Quadra";
pub const ROSEMARY_DEFAULT_PREFIX: &str = "Rosemary default";

const SAVE_DELAY_AFTER_LAST_CHANGE: Duration = Duration::from_millis(500);
const SAVED_PROPERTIES: [&str; 43] = [
    "launch-mode",
    "run-in-background",
    "discord-rich-presence",
    "runtime",
    "runner",
    "custom-runner-path",
    "prefix",
    "renderer",
    "mangohud",
    "refresh-rate",
    "presentation-mode",
    "opengl-swap-interval",
    "title-bar",
    "show-menu-bar",
    "command-key",
    "ctrl-for-edit-shortcuts",
    "plain-ctrl-click",
    "environment-variables",
    "launch-wrapper",
    "fast-flags",
    "preferred-region",
    "vr",
    "vr-mirror",
    "vr-render-scale",
    "vr-zink",
    "death-sound",
    "custom-death-sound",
    "old-character-sounds",
    "cursor-style",
    "custom-cursor",
    "custom-font",
    "custom-mods",
    "hide-launcher-while-playing",
    "open-after-exit",
    "show-playtime",
    "playtime-seconds",
    "notify-server-location",
    "cpu-core-limit",
    "block-telemetry",
    "force-raknet",
    "theme",
    "advanced-settings-visible",
    "system-title-bar",
];
const PROPERTIES_THAT_DECIDE_CAN_PLAY: [&str; 4] = [
    "roblox-installed",
    "studio-installed",
    "rosemary-installed",
    "runner",
];

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Runtime {
    #[default]
    Rosemary,
}

impl Choice for Runtime {
    const ALL: &'static [Self] = &[Self::Rosemary];

    fn label(self) -> &'static str {
        match self {
            Self::Rosemary => "Rosemary",
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    Adwaita,
    Grid,
    Monochrome,
    Aurora,
    Ember,
    Paper,
    Phosphor,
}

impl Choice for Theme {
    const ALL: &'static [Self] = &[
        Self::Adwaita,
        Self::Grid,
        Self::Monochrome,
        Self::Aurora,
        Self::Ember,
        Self::Paper,
        Self::Phosphor,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Adwaita => "Adwaita",
            Self::Grid => "Grid",
            Self::Monochrome => "Monochrome",
            Self::Aurora => "Aurora",
            Self::Ember => "Ember",
            Self::Paper => "Paper",
            Self::Phosphor => "Phosphor",
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
struct SavedSettings {
    launch_mode: LaunchMode,
    run_in_background: bool,
    discord_rich_presence: bool,
    runtime: Runtime,
    runner: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    custom_runner_path: String,
    prefix: String,
    #[serde(rename = "force_opengl", skip_serializing)]
    legacy_force_opengl: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    renderer: Option<Renderer>,
    mangohud: bool,
    refresh_rate: u32,
    presentation_mode: PresentationMode,
    opengl_swap_interval: SwapInterval,
    title_bar: TitleBar,
    show_menu_bar: bool,
    command_key: CommandKey,
    ctrl_for_edit_shortcuts: bool,
    plain_ctrl_click: bool,
    opened_before: bool,
    environment_variables: EnvironmentVariables,
    #[serde(skip_serializing_if = "String::is_empty")]
    launch_wrapper: String,
    fast_flags: FastFlags,
    #[serde(skip_serializing_if = "String::is_empty")]
    preferred_region: String,
    vr: bool,
    vr_mirror: bool,
    vr_render_scale: u32,
    vr_zink: bool,
    death_sound: DeathSound,
    #[serde(skip_serializing_if = "String::is_empty")]
    custom_death_sound: String,
    old_character_sounds: bool,
    cursor_style: CursorStyle,
    #[serde(skip_serializing_if = "String::is_empty")]
    custom_cursor: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    custom_font: String,
    custom_mods: bool,
    hide_launcher_while_playing: bool,
    open_after_exit: bool,
    show_playtime: bool,
    playtime_seconds: u64,
    notify_server_location: bool,
    cpu_core_limit: u32,
    block_telemetry: bool,
    force_raknet: bool,
    theme: Theme,
    advanced_settings_visible: bool,
    system_title_bar: bool,
}

impl Default for SavedSettings {
    fn default() -> Self {
        Self {
            launch_mode: LaunchMode::default(),
            run_in_background: false,
            discord_rich_presence: false,
            runtime: Runtime::default(),
            runner: QUADRA_RUNNER.to_owned(),
            custom_runner_path: String::new(),
            prefix: QUADRA_PREFIX.to_owned(),
            legacy_force_opengl: false,
            renderer: None,
            mangohud: false,
            refresh_rate: USE_MONITOR_REFRESH_RATE,
            presentation_mode: PresentationMode::default(),
            opengl_swap_interval: SwapInterval::default(),
            title_bar: TitleBar::default(),
            show_menu_bar: true,
            command_key: CommandKey::default(),
            ctrl_for_edit_shortcuts: false,
            plain_ctrl_click: true,
            opened_before: false,
            environment_variables: EnvironmentVariables::default(),
            launch_wrapper: String::new(),
            fast_flags: FastFlags::default(),
            preferred_region: String::new(),
            vr: false,
            vr_mirror: true,
            vr_render_scale: DEFAULT_VR_RENDER_SCALE,
            vr_zink: false,
            death_sound: DeathSound::default(),
            custom_death_sound: String::new(),
            old_character_sounds: false,
            cursor_style: CursorStyle::default(),
            custom_cursor: String::new(),
            custom_font: String::new(),
            custom_mods: true,
            hide_launcher_while_playing: false,
            open_after_exit: false,
            show_playtime: true,
            playtime_seconds: 0,
            notify_server_location: false,
            cpu_core_limit: 0,
            block_telemetry: false,
            force_raknet: false,
            theme: Theme::default(),
            advanced_settings_visible: false,
            system_title_bar: false,
        }
    }
}

impl SavedSettings {
    fn chosen_renderer(&self) -> Renderer {
        match self.renderer {
            Some(renderer) => renderer,
            None if self.legacy_force_opengl => Renderer::OpenGl,
            None => Renderer::default(),
        }
    }
}

macro_rules! properties {
    ($($getter:ident $setter:ident: $value:ty = $name:literal,)*) => {
        #[derive(Default)]
        struct Properties {
            $($getter: RefCell<$value>,)*
        }

        impl AppState {
            $(
                pub fn $getter(&self) -> $value {
                    self.0.properties.$getter.borrow().clone()
                }

                pub fn $setter(&self, value: impl Into<$value>) {
                    self.0.properties.$getter.replace(value.into());
                    self.notify($name);
                }
            )*
        }
    };
}

properties! {
    launch_mode set_launch_mode: LaunchMode = "launch-mode",
    run_in_background set_run_in_background: bool = "run-in-background",
    discord_rich_presence set_discord_rich_presence: bool = "discord-rich-presence",
    runtime set_runtime: Runtime = "runtime",
    runner set_runner: String = "runner",
    custom_runner_path set_custom_runner_path: String = "custom-runner-path",
    available_runners set_available_runners: Vec<String> = "available-runners",
    prefix set_prefix: String = "prefix",
    available_prefixes set_available_prefixes: Vec<String> = "available-prefixes",
    darwin_base_installed set_darwin_base_installed: bool = "darwin-base-installed",
    newer_darwin_base set_newer_darwin_base: String = "newer-darwin-base",
    newer_quadra_rosemary set_newer_quadra_rosemary: String = "newer-quadra-rosemary",
    renderer set_renderer: Renderer = "renderer",
    mangohud set_mangohud: bool = "mangohud",
    refresh_rate set_refresh_rate: u32 = "refresh-rate",
    presentation_mode set_presentation_mode: PresentationMode = "presentation-mode",
    opengl_swap_interval set_opengl_swap_interval: SwapInterval = "opengl-swap-interval",
    title_bar set_title_bar: TitleBar = "title-bar",
    show_menu_bar set_show_menu_bar: bool = "show-menu-bar",
    command_key set_command_key: CommandKey = "command-key",
    ctrl_for_edit_shortcuts set_ctrl_for_edit_shortcuts: bool = "ctrl-for-edit-shortcuts",
    plain_ctrl_click set_plain_ctrl_click: bool = "plain-ctrl-click",
    environment_variables set_environment_variables: EnvironmentVariables = "environment-variables",
    launch_wrapper set_launch_wrapper: String = "launch-wrapper",
    fast_flags set_fast_flags: FastFlags = "fast-flags",
    preferred_region set_preferred_region: String = "preferred-region",
    regions set_regions: Regions = "regions",
    server_history set_server_history: ServerHistory = "server-history",
    vr set_vr: bool = "vr",
    vr_mirror set_vr_mirror: bool = "vr-mirror",
    vr_render_scale set_vr_render_scale: u32 = "vr-render-scale",
    vr_zink set_vr_zink: bool = "vr-zink",
    death_sound set_death_sound: DeathSound = "death-sound",
    custom_death_sound set_custom_death_sound: String = "custom-death-sound",
    old_character_sounds set_old_character_sounds: bool = "old-character-sounds",
    cursor_style set_cursor_style: CursorStyle = "cursor-style",
    custom_cursor set_custom_cursor: String = "custom-cursor",
    custom_font set_custom_font: String = "custom-font",
    custom_mods set_custom_mods: bool = "custom-mods",
    hide_launcher_while_playing set_hide_launcher_while_playing: bool = "hide-launcher-while-playing",
    open_after_exit set_open_after_exit: bool = "open-after-exit",
    show_playtime set_show_playtime: bool = "show-playtime",
    playtime_seconds set_playtime_seconds: u64 = "playtime-seconds",
    notify_server_location set_notify_server_location: bool = "notify-server-location",
    cpu_core_limit set_cpu_core_limit: u32 = "cpu-core-limit",
    signed_in set_signed_in: bool = "signed-in",
    block_telemetry set_block_telemetry: bool = "block-telemetry",
    force_raknet set_force_raknet: bool = "force-raknet",
    advanced_settings_visible set_advanced_settings_visible: bool = "advanced-settings-visible",
    available_update set_available_update: String = "available-update",
    update_status set_update_status: String = "update-status",
    vr_status set_vr_status: String = "vr-status",
    first_launch set_first_launch: bool = "first-launch",
    roblox_version set_roblox_version: String = "roblox-version",
    studio_version set_studio_version: String = "studio-version",
    selected_client set_selected_client: Client = "selected-client",
    running_client set_running_client: Client = "running-client",
    busy set_busy: bool = "busy",
    task_description set_task_description: String = "task-description",
    task_progress set_task_progress: f64 = "task-progress",
    error_message set_error_message: String = "error-message",
    running set_running: bool = "running",
    roblox_graphics_started set_roblox_graphics_started: bool = "roblox-graphics-started",
    stopping set_stopping: bool = "stopping",
    roblox_installed set_roblox_installed: bool = "roblox-installed",
    studio_installed set_studio_installed: bool = "studio-installed",
    rosemary_installed set_rosemary_installed: bool = "rosemary-installed",
    can_play set_can_play: bool = "can-play",
    can_open_studio set_can_open_studio: bool = "can-open-studio",
    theme set_theme: Theme = "theme",
    system_prefers_dark set_system_prefers_dark: bool = "system-prefers-dark",
    window_button_layout set_window_button_layout: String = "window-button-layout",
    system_title_bar set_system_title_bar: bool = "system-title-bar",
}

pub type HandlerId = u64;

type PropertyHandler = Rc<dyn Fn(&AppState)>;
type LogHandler = Rc<dyn Fn(&AppState, LogChange<'_>)>;

#[derive(Clone, Copy)]
pub enum LogChange<'a> {
    Cleared,
    Appended(&'a str),
}

#[derive(Default)]
struct Handlers {
    next_id: HandlerId,
    property: Vec<(HandlerId, &'static str, PropertyHandler)>,
    log: Vec<(HandlerId, LogHandler)>,
}

impl Handlers {
    fn next_id(&mut self) -> HandlerId {
        self.next_id += 1;
        self.next_id
    }
}

#[derive(Default)]
struct Inner {
    properties: Properties,
    handlers: RefCell<Handlers>,
    pending_save: RefCell<Option<Source>>,
    log: RefCell<String>,
}

#[derive(Clone, Default)]
pub struct AppState(Rc<Inner>);

impl AppState {
    pub fn load() -> Self {
        let LoadedSettings {
            settings,
            file_existed,
            problem,
            save_repaired_settings,
        } = read_saved_settings();
        let renderer = settings.chosen_renderer();

        let state = Self::default();
        let first_launch = !file_existed && !settings.opened_before;
        let properties = &state.0.properties;
        properties.launch_mode.replace(settings.launch_mode);
        properties
            .run_in_background
            .replace(settings.run_in_background);
        properties
            .discord_rich_presence
            .replace(settings.discord_rich_presence);
        properties.runtime.replace(settings.runtime);
        properties.runner.replace(settings.runner);
        properties
            .custom_runner_path
            .replace(settings.custom_runner_path);
        properties
            .available_runners
            .replace(vec![QUADRA_RUNNER.to_owned()]);
        properties.prefix.replace(settings.prefix);
        properties.available_prefixes.replace(vec![
            QUADRA_PREFIX.to_owned(),
            ROSEMARY_DEFAULT_PREFIX.to_owned(),
        ]);
        properties.renderer.replace(renderer);
        properties.mangohud.replace(settings.mangohud);
        properties.refresh_rate.replace(settings.refresh_rate);
        properties
            .presentation_mode
            .replace(settings.presentation_mode);
        properties
            .opengl_swap_interval
            .replace(settings.opengl_swap_interval);
        properties.title_bar.replace(settings.title_bar);
        properties.show_menu_bar.replace(settings.show_menu_bar);
        properties.command_key.replace(settings.command_key);
        properties
            .ctrl_for_edit_shortcuts
            .replace(settings.ctrl_for_edit_shortcuts);
        properties
            .plain_ctrl_click
            .replace(settings.plain_ctrl_click);
        properties
            .environment_variables
            .replace(settings.environment_variables);
        properties.launch_wrapper.replace(settings.launch_wrapper);
        properties.fast_flags.replace(settings.fast_flags);
        properties
            .preferred_region
            .replace(settings.preferred_region);
        properties.vr.replace(settings.vr);
        properties.vr_mirror.replace(settings.vr_mirror);
        properties.vr_render_scale.replace(
            settings
                .vr_render_scale
                .clamp(*VR_RENDER_SCALE_RANGE.start(), *VR_RENDER_SCALE_RANGE.end()),
        );
        properties.vr_zink.replace(settings.vr_zink);
        properties.death_sound.replace(settings.death_sound);
        properties
            .custom_death_sound
            .replace(settings.custom_death_sound);
        properties
            .old_character_sounds
            .replace(settings.old_character_sounds);
        properties.cursor_style.replace(settings.cursor_style);
        properties.custom_cursor.replace(settings.custom_cursor);
        properties.custom_font.replace(settings.custom_font);
        properties.custom_mods.replace(settings.custom_mods);
        properties
            .hide_launcher_while_playing
            .replace(settings.hide_launcher_while_playing);
        properties.open_after_exit.replace(settings.open_after_exit);
        properties.show_playtime.replace(settings.show_playtime);
        properties
            .playtime_seconds
            .replace(settings.playtime_seconds);
        properties
            .notify_server_location
            .replace(settings.notify_server_location);
        properties.cpu_core_limit.replace(settings.cpu_core_limit);
        properties.block_telemetry.replace(settings.block_telemetry);
        properties.force_raknet.replace(settings.force_raknet);
        properties
            .advanced_settings_visible
            .replace(settings.advanced_settings_visible);
        properties
            .system_title_bar
            .replace(settings.system_title_bar);
        properties.first_launch.replace(first_launch);
        properties.theme.replace(settings.theme);

        state.connect_properties_notify(&SAVED_PROPERTIES, Self::schedule_save);
        state.follow_properties(&PROPERTIES_THAT_DECIDE_CAN_PLAY, Self::update_can_play);
        state.follow_properties(&["vr"], Self::use_opengl_while_in_vr);

        if state.first_launch() || save_repaired_settings {
            state.schedule_save();
        }

        if let Some(problem) = problem {
            let state = state.clone();
            main_loop::idle_once(move || state.report_error(&problem));
        }

        state
    }

    fn notify(&self, property: &'static str) {
        let handlers: Vec<PropertyHandler> = self
            .0
            .handlers
            .borrow()
            .property
            .iter()
            .filter(|(_, followed, _)| *followed == property)
            .map(|(_, _, handler)| handler.clone())
            .collect();
        for handler in handlers {
            handler(self);
        }
    }

    pub fn connect_property_notify(
        &self,
        property: &'static str,
        handler: impl Fn(&Self) + 'static,
    ) -> HandlerId {
        let mut handlers = self.0.handlers.borrow_mut();
        let id = handlers.next_id();
        handlers.property.push((id, property, Rc::new(handler)));
        id
    }

    pub fn connect_properties_notify(
        &self,
        properties: &[&'static str],
        handler: impl Fn(&Self) + 'static,
    ) -> Vec<HandlerId> {
        let handler: PropertyHandler = Rc::new(handler);
        let mut handlers = self.0.handlers.borrow_mut();
        properties
            .iter()
            .map(|property| {
                let id = handlers.next_id();
                handlers.property.push((id, property, handler.clone()));
                id
            })
            .collect()
    }

    pub fn follow_properties(
        &self,
        properties: &[&'static str],
        handler: impl Fn(&Self) + 'static,
    ) {
        handler(self);
        self.connect_properties_notify(properties, handler);
    }

    pub fn disconnect(&self, id: HandlerId) {
        let mut handlers = self.0.handlers.borrow_mut();
        handlers
            .property
            .retain(|(handler_id, _, _)| *handler_id != id);
        handlers.log.retain(|(handler_id, _)| *handler_id != id);
    }

    pub fn report_error(&self, message: &str) {
        eprintln!("{message}");
        self.set_error_message(message);
    }

    pub fn log(&self) -> String {
        self.0.log.borrow().clone()
    }

    pub fn clear_log(&self) {
        self.0.log.borrow_mut().clear();
        self.emit_log_change(LogChange::Cleared);
    }

    pub fn append_log(&self, text: &str) {
        self.0.log.borrow_mut().push_str(text);
        self.emit_log_change(LogChange::Appended(text));
    }

    fn emit_log_change(&self, change: LogChange<'_>) {
        let handlers: Vec<LogHandler> = self
            .0
            .handlers
            .borrow()
            .log
            .iter()
            .map(|(_, handler)| handler.clone())
            .collect();
        for handler in handlers {
            handler(self, change);
        }
    }

    pub fn connect_log_changed(
        &self,
        handler: impl Fn(&Self, LogChange<'_>) + 'static,
    ) -> HandlerId {
        let mut handlers = self.0.handlers.borrow_mut();
        let id = handlers.next_id();
        handlers.log.push((id, Rc::new(handler)));
        id
    }

    pub fn connect_log_appended(&self, handler: impl Fn(&Self, &str) + 'static) -> HandlerId {
        self.connect_log_changed(move |state, change| {
            if let LogChange::Appended(text) = change {
                handler(state, text);
            }
        })
    }

    pub fn save_pending_changes(&self) {
        if let Some(pending_save) = self.0.pending_save.take() {
            pending_save.remove();
            self.save();
        }
    }

    pub fn is_installed(&self, client: Client) -> bool {
        match client {
            Client::Player => self.roblox_installed(),
            Client::Studio => self.studio_installed(),
        }
    }

    pub fn version_of(&self, client: Client) -> String {
        match client {
            Client::Player => self.roblox_version(),
            Client::Studio => self.studio_version(),
        }
    }

    pub fn can_start(&self, client: Client) -> bool {
        match client {
            Client::Player => self.can_play(),
            Client::Studio => self.can_open_studio(),
        }
    }

    pub fn is_running(&self, client: Client) -> bool {
        self.running() && self.running_client() == client
    }

    pub fn current_client(&self) -> Client {
        if self.running() {
            self.running_client()
        } else {
            self.selected_client()
        }
    }

    fn use_opengl_while_in_vr(&self) {
        if self.vr() && self.renderer() != Renderer::OpenGl {
            self.set_renderer(Renderer::OpenGl);
        }
    }

    fn update_can_play(&self) {
        let runner_will_be_installed_on_play = self.runner() == QUADRA_RUNNER;
        let runner_is_ready = self.rosemary_installed() || runner_will_be_installed_on_play;

        let can_play = self.roblox_installed() && runner_is_ready;
        if self.can_play() != can_play {
            self.set_can_play(can_play);
        }

        let can_open_studio = self.studio_installed() && runner_is_ready;
        if self.can_open_studio() != can_open_studio {
            self.set_can_open_studio(can_open_studio);
        }
    }

    fn schedule_save(&self) {
        if let Some(previous_save) = self.0.pending_save.take() {
            previous_save.remove();
        }

        let state = self.clone();
        let pending_save = main_loop::timeout_once(SAVE_DELAY_AFTER_LAST_CHANGE, move || {
            state.0.pending_save.take();
            state.save();
        });

        self.0.pending_save.replace(Some(pending_save));
    }

    fn save(&self) {
        let settings = SavedSettings {
            launch_mode: self.launch_mode(),
            run_in_background: self.run_in_background(),
            discord_rich_presence: self.discord_rich_presence(),
            runtime: self.runtime(),
            runner: self.runner(),
            custom_runner_path: self.custom_runner_path(),
            prefix: self.prefix(),
            legacy_force_opengl: false,
            renderer: Some(self.renderer()),
            mangohud: self.mangohud(),
            refresh_rate: self.refresh_rate(),
            presentation_mode: self.presentation_mode(),
            opengl_swap_interval: self.opengl_swap_interval(),
            title_bar: self.title_bar(),
            show_menu_bar: self.show_menu_bar(),
            command_key: self.command_key(),
            ctrl_for_edit_shortcuts: self.ctrl_for_edit_shortcuts(),
            plain_ctrl_click: self.plain_ctrl_click(),
            opened_before: true,
            environment_variables: self.environment_variables(),
            launch_wrapper: self.launch_wrapper(),
            fast_flags: self.fast_flags(),
            preferred_region: self.preferred_region(),
            vr: self.vr(),
            vr_mirror: self.vr_mirror(),
            vr_render_scale: self.vr_render_scale(),
            vr_zink: self.vr_zink(),
            death_sound: self.death_sound(),
            custom_death_sound: self.custom_death_sound(),
            old_character_sounds: self.old_character_sounds(),
            cursor_style: self.cursor_style(),
            custom_cursor: self.custom_cursor(),
            custom_font: self.custom_font(),
            custom_mods: self.custom_mods(),
            hide_launcher_while_playing: self.hide_launcher_while_playing(),
            open_after_exit: self.open_after_exit(),
            show_playtime: self.show_playtime(),
            playtime_seconds: self.playtime_seconds(),
            notify_server_location: self.notify_server_location(),
            cpu_core_limit: self.cpu_core_limit(),
            block_telemetry: self.block_telemetry(),
            force_raknet: self.force_raknet(),
            theme: self.theme(),
            advanced_settings_visible: self.advanced_settings_visible(),
            system_title_bar: self.system_title_bar(),
        };

        if let Err(error) = write_saved_settings(&settings) {
            eprintln!(
                "Failed to save settings to {}: {error}",
                settings_path().display()
            );
        }
    }
}

pub fn settings_path() -> PathBuf {
    config_directory().join("quadra").join("config.toml")
}

enum SettingsProblem {
    Unparsable,
    InvalidValues(Vec<String>),
}

fn parse_settings(contents: &str) -> (SavedSettings, Option<SettingsProblem>) {
    let Ok(table) = toml::from_str::<toml::Table>(contents) else {
        return (SavedSettings::default(), Some(SettingsProblem::Unparsable));
    };
    let table = hidden_title_bar_as_hidden_menu_bar(table);
    if let Ok(settings) = table.clone().try_into::<SavedSettings>() {
        return (settings, None);
    }

    let (readable, unreadable): (toml::Table, toml::Table) =
        table.into_iter().partition(|(name, value)| {
            toml::Table::from_iter([(name.clone(), value.clone())])
                .try_into::<SavedSettings>()
                .is_ok()
        });
    let settings = readable.try_into().unwrap_or_default();
    let names = unreadable.keys().cloned().collect();
    (settings, Some(SettingsProblem::InvalidValues(names)))
}

fn hidden_title_bar_as_hidden_menu_bar(mut table: toml::Table) -> toml::Table {
    let title_bar_is_hidden =
        table.get("title_bar").and_then(toml::Value::as_str) == Some("hidden");
    if title_bar_is_hidden {
        table.insert("title_bar".to_owned(), "system".into());
        table.insert("show_menu_bar".to_owned(), false.into());
    }
    table
}

struct LoadedSettings {
    settings: SavedSettings,
    file_existed: bool,
    problem: Option<String>,
    save_repaired_settings: bool,
}

impl SettingsProblem {
    fn explanation(&self, kept_copy: Option<&Path>) -> String {
        let what_happened = match self {
            Self::Unparsable => {
                "Quadra couldn't read your settings file, so every setting is back to its default."
                    .to_owned()
            }
            Self::InvalidValues(names) => format!(
                "Some of your settings had values Quadra doesn't understand, so they're back to their defaults: {}.",
                names.join(", ")
            ),
        };
        match kept_copy {
            Some(copy) => format!(
                "{what_happened} A copy of the old file is at {}.",
                copy.display()
            ),
            None => what_happened,
        }
    }
}

fn keep_copy_of_damaged_settings(path: &Path) -> Option<PathBuf> {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    let copy = path.with_extension(format!("toml.damaged-{seconds}"));
    match fs::copy(path, &copy) {
        Ok(_) => Some(copy),
        Err(error) => {
            eprintln!("Couldn't keep a copy of {}: {error}", path.display());
            None
        }
    }
}

fn read_saved_settings() -> LoadedSettings {
    let path = settings_path();

    let contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return LoadedSettings {
                settings: SavedSettings::default(),
                file_existed: false,
                problem: None,
                save_repaired_settings: false,
            };
        }
        Err(error) => {
            return LoadedSettings {
                settings: SavedSettings::default(),
                file_existed: true,
                problem: Some(format!(
                    "Quadra couldn't open your settings file at {} ({error}), so it's using the default settings for now.",
                    path.display()
                )),
                save_repaired_settings: false,
            };
        }
    };

    let (settings, problem) = parse_settings(&contents);
    let kept_copy = problem
        .as_ref()
        .and_then(|_| keep_copy_of_damaged_settings(&path));
    let problem = problem.map(|problem| problem.explanation(kept_copy.as_deref()));
    LoadedSettings {
        settings,
        file_existed: true,
        problem,
        save_repaired_settings: kept_copy.is_some(),
    }
}

fn write_saved_settings(settings: &SavedSettings) -> Result<(), Box<dyn Error>> {
    let path = settings_path();
    let unfinished_path = path.with_extension("toml.partial");

    if let Some(directory) = path.parent() {
        fs::create_dir_all(directory)?;
    }
    fs::write(&unfinished_path, toml::to_string_pretty(settings)?)?;
    fs::rename(&unfinished_path, &path)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_valid_settings_file() {
        let (settings, problem) =
            parse_settings("runner = \"Butter\"\nmangohud = true\nrenderer = \"opengl\"\n");
        assert!(problem.is_none());
        assert_eq!(settings.runner, "Butter");
        assert!(settings.mangohud);
        assert_eq!(settings.renderer, Some(Renderer::OpenGl));
    }

    #[test]
    fn missing_settings_get_their_defaults() {
        let (settings, problem) = parse_settings("mangohud = true\n");
        assert!(problem.is_none());
        assert_eq!(settings.runner, QUADRA_RUNNER);
        assert_eq!(settings.prefix, QUADRA_PREFIX);
        assert!(!settings.discord_rich_presence);
        assert!(settings.vr_mirror);
        assert!(settings.show_menu_bar);
        assert_eq!(settings.vr_render_scale, DEFAULT_VR_RENDER_SCALE);
        assert!(settings.custom_mods);
        assert!(settings.show_playtime);
    }

    #[test]
    fn keeps_the_settings_it_understands_when_one_value_is_unknown() {
        let (settings, problem) =
            parse_settings("runner = \"Butter\"\nmangohud = true\nrenderer = \"vulkan\"\n");
        assert_eq!(settings.runner, "Butter");
        assert!(settings.mangohud);
        assert_eq!(settings.renderer, None);
        assert!(
            matches!(problem, Some(SettingsProblem::InvalidValues(names)) if names == ["renderer"])
        );
    }

    #[test]
    fn falls_back_to_defaults_when_the_file_is_not_toml() {
        let (settings, problem) = parse_settings("this is = = not toml");
        assert!(matches!(problem, Some(SettingsProblem::Unparsable)));
        assert_eq!(settings.runner, QUADRA_RUNNER);
        assert!(!settings.mangohud);
    }

    #[test]
    fn a_hidden_title_bar_becomes_a_system_title_bar_without_menus() {
        let (settings, problem) = parse_settings("title_bar = \"hidden\"\n");
        assert!(problem.is_none());
        assert_eq!(settings.title_bar, TitleBar::System);
        assert!(!settings.show_menu_bar);
    }

    #[test]
    fn ignores_settings_it_does_not_know_about() {
        let (settings, problem) =
            parse_settings("mangohud = true\nsetting_from_a_newer_quadra = 1\n");
        assert!(problem.is_none());
        assert!(settings.mangohud);
    }
}
