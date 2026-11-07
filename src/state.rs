mod choice;
mod environment_variables;
mod fast_flags;
mod launch_mode;
mod rosemary_options;

pub use choice::Choice;
pub use environment_variables::{EnvironmentVariable, EnvironmentVariables};
pub use fast_flags::{FastFlagValue, FastFlags};
pub use launch_mode::LaunchMode;
pub use rosemary_options::{CommandKey, PresentationMode, Renderer, SwapInterval, TitleBar};

use std::error::Error;
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::time::Duration;

use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use serde::{Deserialize, Serialize};

pub const USE_MONITOR_REFRESH_RATE: u32 = 0;
pub const QUADRA_RUNNER: &str = "Quadra";
pub const QUADRA_PREFIX: &str = "Quadra";
pub const ROSEMARY_DEFAULT_PREFIX: &str = "Rosemary default";

const SAVE_DELAY_AFTER_LAST_CHANGE: Duration = Duration::from_millis(500);
const SAVED_PROPERTIES: [&str; 18] = [
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
];
const PROPERTIES_THAT_DECIDE_CAN_PLAY: [&str; 3] =
    ["roblox-installed", "rosemary-installed", "runner"];

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

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct SavedSettings {
    launch_mode: LaunchMode,
    run_in_background: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    discord_rich_presence: Option<bool>,
    runtime: Runtime,
    runner: Option<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    custom_runner_path: String,
    prefix: Option<String>,
    #[serde(rename = "force_opengl", skip_serializing)]
    legacy_force_opengl: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    renderer: Option<Renderer>,
    mangohud: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    refresh_rate: Option<u32>,
    presentation_mode: PresentationMode,
    opengl_swap_interval: SwapInterval,
    title_bar: TitleBar,
    command_key: CommandKey,
    ctrl_for_edit_shortcuts: bool,
    plain_ctrl_click: bool,
    opened_before: bool,
    environment_variables: EnvironmentVariables,
    fast_flags: FastFlags,
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
        first_launch: Cell<bool>,
        #[property(get, set)]
        roblox_version: RefCell<String>,
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
        rosemary_installed: Cell<bool>,
        #[property(get)]
        pub can_play: Cell<bool>,
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
        let settings = read_saved_settings();
        let renderer = settings.chosen_renderer();

        let state: Self = glib::Object::builder()
            .property("launch-mode", settings.launch_mode)
            .property("run-in-background", settings.run_in_background)
            .property(
                "discord-rich-presence",
                settings.discord_rich_presence.unwrap_or(true),
            )
            .property("runtime", settings.runtime)
            .property(
                "runner",
                settings.runner.unwrap_or_else(|| QUADRA_RUNNER.to_owned()),
            )
            .property("custom-runner-path", settings.custom_runner_path)
            .property("available-runners", vec![QUADRA_RUNNER.to_owned()])
            .property(
                "prefix",
                settings.prefix.unwrap_or_else(|| QUADRA_PREFIX.to_owned()),
            )
            .property(
                "available-prefixes",
                vec![QUADRA_PREFIX.to_owned(), ROSEMARY_DEFAULT_PREFIX.to_owned()],
            )
            .property("renderer", renderer)
            .property("mangohud", settings.mangohud)
            .property(
                "refresh-rate",
                settings.refresh_rate.unwrap_or(USE_MONITOR_REFRESH_RATE),
            )
            .property("presentation-mode", settings.presentation_mode)
            .property("opengl-swap-interval", settings.opengl_swap_interval)
            .property("title-bar", settings.title_bar)
            .property("command-key", settings.command_key)
            .property("ctrl-for-edit-shortcuts", settings.ctrl_for_edit_shortcuts)
            .property("plain-ctrl-click", settings.plain_ctrl_click)
            .property("environment-variables", settings.environment_variables)
            .property("fast-flags", settings.fast_flags)
            .property("first-launch", !settings.opened_before)
            .build();

        for property in SAVED_PROPERTIES {
            state.connect_notify_local(Some(property), |state, _| state.schedule_save());
        }

        for property in PROPERTIES_THAT_DECIDE_CAN_PLAY {
            state.connect_notify_local(Some(property), |state, _| state.update_can_play());
        }
        state.update_can_play();

        if state.first_launch() {
            state.schedule_save();
        }

        state
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

    fn update_can_play(&self) {
        let runner_will_be_installed_on_play = self.runner() == QUADRA_RUNNER;
        let can_play = self.roblox_installed()
            && (self.rosemary_installed() || runner_will_be_installed_on_play);
        let could_play = self.imp().can_play.replace(can_play);
        if could_play != can_play {
            self.notify_can_play();
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
            discord_rich_presence: Some(self.discord_rich_presence()),
            runtime: self.runtime(),
            runner: Some(self.runner()),
            custom_runner_path: self.custom_runner_path(),
            prefix: Some(self.prefix()),
            legacy_force_opengl: false,
            renderer: Some(self.renderer()),
            mangohud: self.mangohud(),
            refresh_rate: Some(self.refresh_rate())
                .filter(|&rate| rate != USE_MONITOR_REFRESH_RATE),
            presentation_mode: self.presentation_mode(),
            opengl_swap_interval: self.opengl_swap_interval(),
            title_bar: self.title_bar(),
            command_key: self.command_key(),
            ctrl_for_edit_shortcuts: self.ctrl_for_edit_shortcuts(),
            plain_ctrl_click: self.plain_ctrl_click(),
            opened_before: true,
            environment_variables: self.environment_variables(),
            fast_flags: self.fast_flags(),
        };

        if let Err(error) = write_saved_settings(&settings) {
            eprintln!(
                "Failed to save settings to {}: {error}",
                settings_path().display()
            );
        }
    }
}

fn settings_path() -> PathBuf {
    glib::user_config_dir().join("quadra").join("config.toml")
}

fn read_saved_settings() -> SavedSettings {
    let path = settings_path();

    let contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == ErrorKind::NotFound => return SavedSettings::default(),
        Err(error) => {
            eprintln!("Failed to read settings from {}: {error}", path.display());
            return SavedSettings::default();
        }
    };

    toml::from_str(&contents).unwrap_or_else(|error| {
        eprintln!("Ignoring invalid settings in {}: {error}", path.display());
        SavedSettings::default()
    })
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
