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

use std::error::Error;
use std::fs;
use std::io::ErrorKind;
use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use serde::{Deserialize, Serialize};

pub const USE_MONITOR_REFRESH_RATE: u32 = 0;
pub const DEFAULT_VR_RENDER_SCALE: u32 = 70;
pub const VR_RENDER_SCALE_RANGE: RangeInclusive<u32> = 40..=150;
pub const QUADRA_RUNNER: &str = "Quadra";
pub const QUADRA_PREFIX: &str = "Quadra";
pub const ROSEMARY_DEFAULT_PREFIX: &str = "Rosemary default";

const SAVE_DELAY_AFTER_LAST_CHANGE: Duration = Duration::from_millis(500);
const SAVED_PROPERTIES: [&str; 37] = [
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
    "command-key",
    "ctrl-for-edit-shortcuts",
    "plain-ctrl-click",
    "environment-variables",
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
];
const PROPERTIES_THAT_DECIDE_CAN_PLAY: [&str; 4] = [
    "roblox-installed",
    "studio-installed",
    "rosemary-installed",
    "runner",
];

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, glib::Enum, Serialize, Deserialize)]
#[enum_type(name = "QuadraRuntime")]
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
    command_key: CommandKey,
    ctrl_for_edit_shortcuts: bool,
    plain_ctrl_click: bool,
    opened_before: bool,
    environment_variables: EnvironmentVariables,
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
}

impl Default for SavedSettings {
    fn default() -> Self {
        Self {
            launch_mode: LaunchMode::default(),
            run_in_background: false,
            discord_rich_presence: true,
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
            command_key: CommandKey::default(),
            ctrl_for_edit_shortcuts: false,
            plain_ctrl_click: false,
            opened_before: false,
            environment_variables: EnvironmentVariables::default(),
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

const LOG_CLEARED: &str = "log-cleared";
const LOG_APPENDED: &str = "log-appended";

mod imp {
    use super::*;
    use glib::subclass::Signal;
    use std::cell::{Cell, RefCell};
    use std::sync::OnceLock;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::AppState)]
    pub struct AppState {
        #[property(get, set, builder(LaunchMode::default()))]
        launch_mode: Cell<LaunchMode>,
        #[property(get, set)]
        run_in_background: Cell<bool>,
        #[property(get, set)]
        discord_rich_presence: Cell<bool>,
        #[property(get, set, builder(Runtime::default()))]
        runtime: Cell<Runtime>,
        #[property(get, set)]
        runner: RefCell<String>,
        #[property(get, set)]
        custom_runner_path: RefCell<String>,
        #[property(get, set)]
        available_runners: RefCell<Vec<String>>,
        #[property(get, set)]
        prefix: RefCell<String>,
        #[property(get, set)]
        available_prefixes: RefCell<Vec<String>>,
        #[property(get, set)]
        darwin_base_installed: Cell<bool>,
        #[property(get, set, builder(Renderer::default()))]
        renderer: Cell<Renderer>,
        #[property(get, set)]
        mangohud: Cell<bool>,
        #[property(get, set)]
        refresh_rate: Cell<u32>,
        #[property(get, set, builder(PresentationMode::default()))]
        presentation_mode: Cell<PresentationMode>,
        #[property(get, set, builder(SwapInterval::default()))]
        opengl_swap_interval: Cell<SwapInterval>,
        #[property(get, set, builder(TitleBar::default()))]
        title_bar: Cell<TitleBar>,
        #[property(get, set, builder(CommandKey::default()))]
        command_key: Cell<CommandKey>,
        #[property(get, set)]
        ctrl_for_edit_shortcuts: Cell<bool>,
        #[property(get, set)]
        plain_ctrl_click: Cell<bool>,
        #[property(get, set)]
        environment_variables: RefCell<EnvironmentVariables>,
        #[property(get, set)]
        fast_flags: RefCell<FastFlags>,
        #[property(get, set)]
        preferred_region: RefCell<String>,
        #[property(get, set)]
        regions: RefCell<Regions>,
        #[property(get, set)]
        server_history: RefCell<ServerHistory>,
        #[property(get, set)]
        vr: Cell<bool>,
        #[property(get, set)]
        vr_mirror: Cell<bool>,
        #[property(get, set)]
        vr_render_scale: Cell<u32>,
        #[property(get, set)]
        vr_zink: Cell<bool>,
        #[property(get, set, builder(DeathSound::default()))]
        death_sound: Cell<DeathSound>,
        #[property(get, set)]
        custom_death_sound: RefCell<String>,
        #[property(get, set)]
        old_character_sounds: Cell<bool>,
        #[property(get, set, builder(CursorStyle::default()))]
        cursor_style: Cell<CursorStyle>,
        #[property(get, set)]
        custom_cursor: RefCell<String>,
        #[property(get, set)]
        custom_font: RefCell<String>,
        #[property(get, set)]
        custom_mods: Cell<bool>,
        #[property(get, set)]
        hide_launcher_while_playing: Cell<bool>,
        #[property(get, set)]
        open_after_exit: Cell<bool>,
        #[property(get, set)]
        show_playtime: Cell<bool>,
        #[property(get, set)]
        playtime_seconds: Cell<u64>,
        #[property(get, set)]
        notify_server_location: Cell<bool>,
        #[property(get, set)]
        cpu_core_limit: Cell<u32>,
        #[property(get, set)]
        signed_in: Cell<bool>,
        #[property(get, set)]
        block_telemetry: Cell<bool>,
        #[property(get, set)]
        available_update: RefCell<String>,
        #[property(get, set)]
        update_status: RefCell<String>,
        #[property(get, set)]
        vr_status: RefCell<String>,
        #[property(get, set)]
        first_launch: Cell<bool>,
        #[property(get, set)]
        roblox_version: RefCell<String>,
        #[property(get, set)]
        studio_version: RefCell<String>,
        #[property(get, set, builder(Client::default()))]
        selected_client: Cell<Client>,
        #[property(get, set, builder(Client::default()))]
        running_client: Cell<Client>,
        #[property(get, set)]
        busy: Cell<bool>,
        #[property(get, set)]
        task_description: RefCell<String>,
        #[property(get, set)]
        task_progress: Cell<f64>,
        #[property(get, set)]
        error_message: RefCell<String>,
        #[property(get, set)]
        running: Cell<bool>,
        #[property(get, set)]
        roblox_graphics_started: Cell<bool>,
        #[property(get, set)]
        stopping: Cell<bool>,
        #[property(get, set)]
        roblox_installed: Cell<bool>,
        #[property(get, set)]
        studio_installed: Cell<bool>,
        #[property(get, set)]
        rosemary_installed: Cell<bool>,
        #[property(get)]
        pub can_play: Cell<bool>,
        #[property(get)]
        pub can_open_studio: Cell<bool>,
        pub pending_save: RefCell<Option<glib::SourceId>>,
        pub log: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AppState {
        const NAME: &'static str = "QuadraAppState";
        type Type = super::AppState;
    }

    #[glib::derived_properties]
    impl ObjectImpl for AppState {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder(LOG_CLEARED).build(),
                    Signal::builder(LOG_APPENDED)
                        .param_types([String::static_type()])
                        .build(),
                ]
            })
        }
    }
}

glib::wrapper! {
    pub struct AppState(ObjectSubclass<imp::AppState>);
}

impl AppState {
    pub fn load() -> Self {
        let LoadedSettings {
            settings,
            file_existed,
            problem,
            save_repaired_settings,
        } = read_saved_settings();
        let renderer = settings.chosen_renderer();

        let state: Self = glib::Object::builder()
            .property("launch-mode", settings.launch_mode)
            .property("run-in-background", settings.run_in_background)
            .property("discord-rich-presence", settings.discord_rich_presence)
            .property("runtime", settings.runtime)
            .property("runner", settings.runner)
            .property("custom-runner-path", settings.custom_runner_path)
            .property("available-runners", vec![QUADRA_RUNNER.to_owned()])
            .property("prefix", settings.prefix)
            .property(
                "available-prefixes",
                vec![QUADRA_PREFIX.to_owned(), ROSEMARY_DEFAULT_PREFIX.to_owned()],
            )
            .property("renderer", renderer)
            .property("mangohud", settings.mangohud)
            .property("refresh-rate", settings.refresh_rate)
            .property("presentation-mode", settings.presentation_mode)
            .property("opengl-swap-interval", settings.opengl_swap_interval)
            .property("title-bar", settings.title_bar)
            .property("command-key", settings.command_key)
            .property("ctrl-for-edit-shortcuts", settings.ctrl_for_edit_shortcuts)
            .property("plain-ctrl-click", settings.plain_ctrl_click)
            .property("environment-variables", settings.environment_variables)
            .property("fast-flags", settings.fast_flags)
            .property("preferred-region", settings.preferred_region)
            .property("vr", settings.vr)
            .property("vr-mirror", settings.vr_mirror)
            .property(
                "vr-render-scale",
                settings
                    .vr_render_scale
                    .clamp(*VR_RENDER_SCALE_RANGE.start(), *VR_RENDER_SCALE_RANGE.end()),
            )
            .property("vr-zink", settings.vr_zink)
            .property("death-sound", settings.death_sound)
            .property("custom-death-sound", settings.custom_death_sound)
            .property("old-character-sounds", settings.old_character_sounds)
            .property("cursor-style", settings.cursor_style)
            .property("custom-cursor", settings.custom_cursor)
            .property("custom-font", settings.custom_font)
            .property("custom-mods", settings.custom_mods)
            .property(
                "hide-launcher-while-playing",
                settings.hide_launcher_while_playing,
            )
            .property("open-after-exit", settings.open_after_exit)
            .property("show-playtime", settings.show_playtime)
            .property("playtime-seconds", settings.playtime_seconds)
            .property("notify-server-location", settings.notify_server_location)
            .property("cpu-core-limit", settings.cpu_core_limit)
            .property("block-telemetry", settings.block_telemetry)
            .property("first-launch", !file_existed && !settings.opened_before)
            .build();

        state.connect_properties_notify(&SAVED_PROPERTIES, Self::schedule_save);
        state.follow_properties(&PROPERTIES_THAT_DECIDE_CAN_PLAY, Self::update_can_play);

        if state.first_launch() || save_repaired_settings {
            state.schedule_save();
        }

        if let Some(problem) = problem {
            let state = state.clone();
            glib::idle_add_local_once(move || state.report_error(&problem));
        }

        state
    }

    pub fn connect_properties_notify(
        &self,
        properties: &[&str],
        handler: impl Fn(&Self) + 'static,
    ) -> Vec<glib::SignalHandlerId> {
        let handler = Rc::new(handler);
        properties
            .iter()
            .map(|property| {
                let handler = handler.clone();
                self.connect_notify_local(Some(property), move |state, _| handler(state))
            })
            .collect()
    }

    pub fn follow_properties(&self, properties: &[&str], handler: impl Fn(&Self) + 'static) {
        handler(self);
        self.connect_properties_notify(properties, handler);
    }

    pub fn report_error(&self, message: &str) {
        eprintln!("{message}");
        self.set_error_message(message);
    }

    pub fn log(&self) -> String {
        self.imp().log.borrow().clone()
    }

    pub fn clear_log(&self) {
        self.imp().log.borrow_mut().clear();
        self.emit_by_name::<()>(LOG_CLEARED, &[]);
    }

    pub fn append_log(&self, text: &str) {
        self.imp().log.borrow_mut().push_str(text);
        self.emit_by_name::<()>(LOG_APPENDED, &[&text]);
    }

    pub fn connect_log_cleared(&self, handler: impl Fn(&Self) + 'static) -> glib::SignalHandlerId {
        self.connect_local(LOG_CLEARED, false, move |values| {
            let state = values[0]
                .get::<Self>()
                .expect("signal is emitted by AppState");
            handler(&state);
            None
        })
    }

    pub fn connect_log_appended(
        &self,
        handler: impl Fn(&Self, &str) + 'static,
    ) -> glib::SignalHandlerId {
        self.connect_local(LOG_APPENDED, false, move |values| {
            let state = values[0]
                .get::<Self>()
                .expect("signal is emitted by AppState");
            let text = values[1].get::<&str>().expect("log-appended carries text");
            handler(&state, text);
            None
        })
    }

    pub fn save_pending_changes(&self) {
        if let Some(pending_save) = self.imp().pending_save.take() {
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

    fn update_can_play(&self) {
        let runner_will_be_installed_on_play = self.runner() == QUADRA_RUNNER;
        let runner_is_ready = self.rosemary_installed() || runner_will_be_installed_on_play;

        let can_play = self.roblox_installed() && runner_is_ready;
        let could_play = self.imp().can_play.replace(can_play);
        if could_play != can_play {
            self.notify_can_play();
        }

        let can_open_studio = self.studio_installed() && runner_is_ready;
        let could_open_studio = self.imp().can_open_studio.replace(can_open_studio);
        if could_open_studio != can_open_studio {
            self.notify_can_open_studio();
        }
    }

    fn schedule_save(&self) {
        if let Some(previous_save) = self.imp().pending_save.take() {
            previous_save.remove();
        }

        let state = self.clone();
        let pending_save = glib::timeout_add_local_once(SAVE_DELAY_AFTER_LAST_CHANGE, move || {
            state.imp().pending_save.take();
            state.save();
        });

        self.imp().pending_save.replace(Some(pending_save));
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
            command_key: self.command_key(),
            ctrl_for_edit_shortcuts: self.ctrl_for_edit_shortcuts(),
            plain_ctrl_click: self.plain_ctrl_click(),
            opened_before: true,
            environment_variables: self.environment_variables(),
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
    glib::user_config_dir().join("quadra").join("config.toml")
}

enum SettingsProblem {
    Unparsable,
    InvalidValues(Vec<String>),
}

fn parse_settings(contents: &str) -> (SavedSettings, Option<SettingsProblem>) {
    let Ok(table) = toml::from_str::<toml::Table>(contents) else {
        return (SavedSettings::default(), Some(SettingsProblem::Unparsable));
    };
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
        assert!(settings.discord_rich_presence);
        assert!(settings.vr_mirror);
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
    fn ignores_settings_it_does_not_know_about() {
        let (settings, problem) =
            parse_settings("mangohud = true\nsetting_from_a_newer_quadra = 1\n");
        assert!(problem.is_none());
        assert!(settings.mangohud);
    }
}
