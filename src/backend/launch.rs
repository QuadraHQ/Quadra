use super::runners::Runner;
use super::{fast_flags, live_log, mangohud, paths, prefix, roblox};
use crate::state::{
    AppState, CommandKey, PresentationMode, SwapInterval, TitleBar, USE_MONITOR_REFRESH_RATE,
};
use anyhow::Context;
use gtk::glib;
use std::cell::RefCell;
use std::fs::{self, File};
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

const LOG_FILE: &str = "roblox.log";
const FORCE_STOP_DELAY: Duration = Duration::from_secs(10);

struct RunningRoblox {
    process_group: i32,
    rosemary: PathBuf,
    prefix_directory: Option<PathBuf>,
}

thread_local! {
    static RUNNING_ROBLOX: RefCell<Option<RunningRoblox>> = const { RefCell::new(None) };
}

pub fn start_roblox(state: &AppState, runner: &Runner) -> anyhow::Result<()> {
    fast_flags::write_client_settings(&state.fast_flags())?;

    fs::create_dir_all(paths::logs_directory())?;
    let log_path = paths::logs_directory().join(LOG_FILE);
    let log = File::create(&log_path)?;
    let log_reader = File::open(&log_path)?;

    let mut command = roblox_command(state, runner);
    command
        .arg(roblox::app_path())
        .envs(rosemary_environment(state))
        .stdin(Stdio::null())
        .stdout(log.try_clone()?)
        .stderr(log)
        .process_group(0);

    let mut child = command
        .spawn()
        .with_context(|| format!("Couldn't start {}", runner.executable.display()))?;
    RUNNING_ROBLOX.replace(Some(RunningRoblox {
        process_group: child.id() as i32,
        rosemary: runner.executable.clone(),
        prefix_directory: prefix::directory_for(&state.prefix()),
    }));
    state.set_roblox_graphics_started(false);
    state.set_running(true);

    let roblox_exited = Arc::new(AtomicBool::new(false));
    live_log::follow(state, log_reader, roblox_exited.clone());

    let (exit_sender, exit_receiver) = async_channel::bounded(1);
    std::thread::spawn(move || {
        let _ = child.wait();
        roblox_exited.store(true, Ordering::Release);
        let _ = exit_sender.send_blocking(());
    });

    let state = state.clone();
    glib::spawn_future_local(async move {
        let _ = exit_receiver.recv().await;
        RUNNING_ROBLOX.replace(None);
        state.set_stopping(false);
        state.set_running(false);
        state.set_roblox_graphics_started(false);
        super::refresh(&state);
    });

    Ok(())
}

fn roblox_command(state: &AppState, runner: &Runner) -> Command {
    if !state.mangohud() {
        return Command::new(&runner.executable);
    }

    match mangohud::executable() {
        Some(mangohud) => {
            let mut command = Command::new(mangohud);
            command.arg(&runner.executable);
            command
        }
        None => {
            let mut command = Command::new(&runner.executable);
            command.env("MANGOHUD", "1");
            command
        }
    }
}

pub fn stop_roblox() {
    RUNNING_ROBLOX.with_borrow(|running_roblox| {
        let Some(running_roblox) = running_roblox else {
            return;
        };

        let process_group = running_roblox.process_group;
        signal_process_group(process_group, libc::SIGTERM);
        glib::timeout_add_local_once(FORCE_STOP_DELAY, move || {
            if is_still_running(process_group) {
                signal_process_group(process_group, libc::SIGKILL);
            }
        });

        let mut stop_prefix_server = Command::new(&running_roblox.rosemary);
        stop_prefix_server
            .args(["server", "--stop"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if let Some(prefix_directory) = &running_roblox.prefix_directory {
            stop_prefix_server.env("ROSEMARY_PREFIX", prefix_directory);
        }
        if let Ok(mut stopping) = stop_prefix_server.spawn() {
            std::thread::spawn(move || stopping.wait());
        }
    });
}

fn is_still_running(process_group: i32) -> bool {
    RUNNING_ROBLOX.with_borrow(|running_roblox| {
        running_roblox
            .as_ref()
            .is_some_and(|running_roblox| running_roblox.process_group == process_group)
    })
}

fn signal_process_group(process_group: i32, signal: libc::c_int) {
    unsafe {
        libc::kill(-process_group, signal);
    }
}

fn rosemary_environment(state: &AppState) -> Vec<(String, String)> {
    let mut environment: Vec<(&str, String)> = Vec::new();

    if let Some(prefix_directory) = prefix::directory_for(&state.prefix()) {
        environment.push(("ROSEMARY_PREFIX", prefix_directory.display().to_string()));
        environment.push(("ROSEMARY_PREFIX_SETUP", "download".to_owned()));
    }

    if state.force_opengl() {
        environment.push(("VK_DRIVER_FILES", "/dev/null".to_owned()));
        if let Some(swap_interval) = swap_interval_value(state.opengl_swap_interval()) {
            environment.push(("ROSEMARY_SWAP_INTERVAL", swap_interval.to_owned()));
        }
    } else if let Some(presentation_mode) = presentation_mode_value(state.presentation_mode()) {
        environment.push((
            "ROSEMARY_METAL_PRESENTATION_MODE",
            presentation_mode.to_owned(),
        ));
    }

    if state.refresh_rate() != USE_MONITOR_REFRESH_RATE {
        environment.push((
            "ROSEMARY_SCREEN_REFRESH_RATE",
            state.refresh_rate().to_string(),
        ));
    }

    environment.push((
        "ROSEMARY_DECORATIONS",
        decorations_value(state.title_bar()).to_owned(),
    ));
    environment.push((
        "ROSEMARY_COMMAND_KEY",
        command_key_value(state.command_key(), state.ctrl_for_edit_shortcuts()),
    ));
    environment.push((
        "ROSEMARY_PLAIN_CTRL_CLICK",
        if state.plain_ctrl_click() { "1" } else { "0" }.to_owned(),
    ));

    let mut environment: Vec<(String, String)> = environment
        .into_iter()
        .map(|(name, value)| (name.to_owned(), value))
        .collect();

    environment.extend(
        state
            .environment_variables()
            .enabled()
            .map(|variable| (variable.name.clone(), variable.value.clone())),
    );

    environment
}

fn presentation_mode_value(mode: PresentationMode) -> Option<&'static str> {
    match mode {
        PresentationMode::Default => None,
        PresentationMode::Immediate => Some("immediate"),
        PresentationMode::Fifo => Some("fifo"),
        PresentationMode::FifoRelaxed => Some("fifo_relaxed"),
        PresentationMode::Mailbox => Some("mailbox"),
    }
}

fn swap_interval_value(interval: SwapInterval) -> Option<&'static str> {
    match interval {
        SwapInterval::Default => None,
        SwapInterval::Immediate => Some("0"),
        SwapInterval::VSync => Some("1"),
        SwapInterval::AdaptiveVSync => Some("-1"),
    }
}

fn decorations_value(title_bar: TitleBar) -> &'static str {
    match title_bar {
        TitleBar::MacStyle => "client",
        TitleBar::System => "server",
    }
}

fn command_key_value(command_key: CommandKey, ctrl_for_edit_shortcuts: bool) -> String {
    let key = match command_key {
        CommandKey::Super => "super",
        CommandKey::Ctrl => "ctrl",
        CommandKey::Alt => "alt",
    };
    if ctrl_for_edit_shortcuts && command_key != CommandKey::Ctrl {
        format!("{key}+ctrl:edit")
    } else {
        key.to_owned()
    }
}
