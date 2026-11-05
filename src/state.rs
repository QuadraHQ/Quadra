mod choice;
mod rosemary_options;

pub use choice::Choice;
pub use rosemary_options::{CommandKey, PresentationMode, SwapInterval, TitleBar};

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

const SAVE_DELAY_AFTER_LAST_CHANGE: Duration = Duration::from_millis(500);
const SAVED_PROPERTIES: [&str; 9] = [
    "runtime",
    "force-opengl",
    "refresh-rate",
    "presentation-mode",
    "opengl-swap-interval",
    "title-bar",
    "command-key",
    "ctrl-for-edit-shortcuts",
    "plain-ctrl-click",
];
const PROPERTIES_THAT_DECIDE_CAN_PLAY: [&str; 2] = ["roblox-installed", "rosemary-installed"];

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
    runtime: Runtime,
    force_opengl: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    refresh_rate: Option<u32>,
    presentation_mode: PresentationMode,
    opengl_swap_interval: SwapInterval,
    title_bar: TitleBar,
    command_key: CommandKey,
    ctrl_for_edit_shortcuts: bool,
    plain_ctrl_click: bool,
}

mod imp {
    use super::*;
    use std::cell::{Cell, RefCell};

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::AppState)]
    pub struct AppState {
        #[property(get, set, builder(Runtime::default()))]
        runtime: Cell<Runtime>,
        #[property(get, set)]
        force_opengl: Cell<bool>,
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
        running: Cell<bool>,
        #[property(get, set)]
        roblox_installed: Cell<bool>,
        #[property(get, set)]
        rosemary_installed: Cell<bool>,
        #[property(get)]
        pub can_play: Cell<bool>,
        pub pending_save: RefCell<Option<glib::SourceId>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AppState {
        const NAME: &'static str = "QuadraAppState";
        type Type = super::AppState;
    }

    #[glib::derived_properties]
    impl ObjectImpl for AppState {}
}

glib::wrapper! {
    pub struct AppState(ObjectSubclass<imp::AppState>);
}

impl AppState {
    pub fn load() -> Self {
        let settings = read_saved_settings();

        let state: Self = glib::Object::builder()
            .property("runtime", settings.runtime)
            .property("force-opengl", settings.force_opengl)
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
            .build();

        for property in SAVED_PROPERTIES {
            state.connect_notify_local(Some(property), |state, _| state.schedule_save());
        }

        for property in PROPERTIES_THAT_DECIDE_CAN_PLAY {
            state.connect_notify_local(Some(property), |state, _| state.update_can_play());
        }
        state.update_can_play();

        state
    }

    pub fn save_pending_changes(&self) {
        if let Some(pending_save) = self.imp().pending_save.take() {
            pending_save.remove();
            self.save();
        }
    }

    fn update_can_play(&self) {
        let can_play = self.roblox_installed() && self.rosemary_installed();
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
            runtime: self.runtime(),
            force_opengl: self.force_opengl(),
            refresh_rate: Some(self.refresh_rate())
                .filter(|&rate| rate != USE_MONITOR_REFRESH_RATE),
            presentation_mode: self.presentation_mode(),
            opengl_swap_interval: self.opengl_swap_interval(),
            title_bar: self.title_bar(),
            command_key: self.command_key(),
            ctrl_for_edit_shortcuts: self.ctrl_for_edit_shortcuts(),
            plain_ctrl_click: self.plain_ctrl_click(),
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
