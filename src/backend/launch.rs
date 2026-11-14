use super::launch_request::StudioTarget;
use super::runners::Runner;
use super::vr::{self, VrSession};
use super::{
    cpu_limit, fast_flags, live_log, mods, paths, prefix, roblox, telemetry, transport_patch,
};
use crate::state::{
    AppState, Client, CommandKey, PresentationMode, Renderer, SwapInterval, TitleBar,
    USE_MONITOR_REFRESH_RATE,
};
use crate::utils::{find_in_path, is_executable_file};
use anyhow::Context;
use gtk::glib;
use std::cell::RefCell;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{self, Write};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

const LOG_FILE: &str = "roblox.log";
const FORCE_STOP_DELAY: Duration = Duration::from_secs(10);
const PROTOCOL_STRING_ARGUMENT: &str = "-protocolString";
const VULKAN_DRIVERS_VARIABLE: &str = "VK_DRIVER_FILES";

struct RunningRoblox {
    process_group: i32,
    rosemary: PathBuf,
    prefix_directory: Option<PathBuf>,
}

thread_local! {
    static RUNNING_ROBLOX: RefCell<Option<RunningRoblox>> = const { RefCell::new(None) };
}

pub fn start_roblox(
    state: &AppState,
    runner: &Runner,
    launch_uri: Option<&str>,
    vr: Option<&Result<VrSession, String>>,
) -> anyhow::Result<()> {
    let vr_session = vr.and_then(|vr| vr.as_ref().ok());
    let fast_flags = match vr_session {
        Some(_) => vr::with_opengl_renderer(state.fast_flags()),
        None => state.fast_flags(),
    };
    fast_flags::write_client_settings(&fast_flags)?;
    transport_patch::prepare(state.force_raknet());

    let mut command = wrapped_command(state, &runner.executable)?;
    command.arg(roblox::app_path(Client::Player));
    if let Some(launch_uri) = launch_uri {
        command.args([PROTOCOL_STRING_ARGUMENT, launch_uri]);
    }
    command.envs(rosemary_environment(state, vr_session));

    run(state, Client::Player, runner, command, vr.map(vr_log_line))
}

pub fn start_studio(
    state: &AppState,
    runner: &Runner,
    target: &StudioTarget,
) -> anyhow::Result<()> {
    let mut command = wrapped_command(state, &runner.executable)?;
    command
        .arg(roblox::app_path(Client::Studio))
        .args(studio_arguments(state, target))
        .envs(rosemary_environment(state, None));

    run(state, Client::Studio, runner, command, None)
}

fn run(
    state: &AppState,
    client: Client,
    runner: &Runner,
    mut command: Command,
    first_log_line: Option<String>,
) -> anyhow::Result<()> {
    fs::create_dir_all(paths::logs_directory())?;
    let log_path = paths::logs_directory().join(LOG_FILE);
    let mut log = File::create(&log_path)?;
    if let Some(first_log_line) = first_log_line {
        writeln!(log, "{first_log_line}")?;
    }
    let log_reader = File::open(&log_path)?;

    command
        .stdin(Stdio::null())
        .stdout(log.try_clone()?)
        .stderr(log)
        .process_group(0);
    cpu_limit::limit(&mut command, state.cpu_core_limit());
    end_when_quadra_ends(&mut command);
    let started_at = Instant::now();

    let mut child = command
        .spawn()
        .with_context(|| format!("Couldn't start {}", command.get_program().display()))?;
    RUNNING_ROBLOX.replace(Some(RunningRoblox {
        process_group: child.id() as i32,
        rosemary: runner.executable.clone(),
        prefix_directory: prefix::directory_for(&state.prefix()),
    }));
    state.set_roblox_graphics_started(false);
    state.set_running_client(client);
    state.set_running(true);

    let roblox_exited = Arc::new(AtomicBool::new(false));
    live_log::follow(state, log_reader, roblox_exited.clone());

    let (exit_sender, exit_receiver) = async_channel::bounded(1);
    std::thread::spawn(move || {
        let exited_cleanly = child.wait().is_ok_and(|status| status.success());
        roblox_exited.store(true, Ordering::Release);
        let _ = exit_sender.send_blocking(exited_cleanly);
    });

    let state = state.clone();
    glib::spawn_future_local(async move {
        let exited_cleanly = exit_receiver.recv().await.unwrap_or(true);
        RUNNING_ROBLOX.replace(None);
        let crashed = !exited_cleanly && !state.stopping();
        if client == Client::Player {
            state.set_playtime_seconds(state.playtime_seconds() + started_at.elapsed().as_secs());
            report_mods_set_aside(&state, crashed);
        }
        state.set_stopping(false);
        state.set_running(false);
        state.set_roblox_graphics_started(false);
        super::refresh(&state);
    });

    Ok(())
}

fn report_mods_set_aside(state: &AppState, crashed: bool) {
    let set_aside = mods::finish_session(crashed);
    if !set_aside.is_empty() {
        state.report_error(&format!(
            "Roblox crashed soon after starting with new mods, so Quadra set them aside: {}. \
             You can turn them back on in Settings.",
            set_aside.join(", ")
        ));
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

pub fn launch_wrapper_problem(wrapper: &str) -> Option<String> {
    match launch_wrapper_words(wrapper) {
        Err(error) => Some(error.to_string()),
        Ok(words) => {
            let program = words.first()?;
            let found = if program.contains('/') {
                is_executable_file(Path::new(program))
            } else {
                find_in_path(program).is_some()
            };
            (!found).then(|| format!("{program} isn't installed, or isn't in PATH."))
        }
    }
}

fn wrapped_command(state: &AppState, rosemary: &Path) -> anyhow::Result<Command> {
    let words = launch_wrapper_words(&state.launch_wrapper())?;
    let Some((program, arguments)) = words.split_first() else {
        return Ok(Command::new(rosemary));
    };
    let mut command = Command::new(program);
    command.args(arguments).arg(rosemary);
    Ok(command)
}

fn launch_wrapper_words(wrapper: &str) -> anyhow::Result<Vec<String>> {
    shlex::split(wrapper).context("The launch wrapper has a quote that's never closed.")
}

fn end_when_quadra_ends(command: &mut Command) {
    let quadra = std::process::id() as libc::pid_t;
    unsafe {
        command.pre_exec(move || {
            if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM as libc::c_ulong) == -1 {
                return Err(io::Error::last_os_error());
            }
            if libc::getppid() != quadra {
                libc::raise(libc::SIGTERM);
            }
            Ok(())
        });
    }
}

fn rosemary_environment(state: &AppState, vr_session: Option<&VrSession>) -> Vec<(String, String)> {
    let mut environment: Vec<(&str, String)> = Vec::new();

    if let Some(prefix_directory) = prefix::directory_for(&state.prefix()) {
        environment.push(("ROSEMARY_PREFIX", prefix_directory.display().to_string()));
        environment.push(("ROSEMARY_PREFIX_SETUP", "download".to_owned()));
    }

    if state.renderer() == Renderer::OpenGl {
        environment.push((VULKAN_DRIVERS_VARIABLE, "/dev/null".to_owned()));
        if let Some(swap_interval) = swap_interval_value(state.opengl_swap_interval()) {
            environment.push(("ROSEMARY_SWAP_INTERVAL", swap_interval.to_owned()));
        }
    } else if let Some(presentation_mode) = presentation_mode_value(state.presentation_mode()) {
        environment.push((
            "ROSEMARY_METAL_PRESENTATION_MODE",
            presentation_mode.to_owned(),
        ));
    }

    if state.mangohud() {
        environment.push(("MANGOHUD", "1".to_owned()));
    }

    if state.refresh_rate() != USE_MONITOR_REFRESH_RATE {
        environment.push((
            "ROSEMARY_SCREEN_REFRESH_RATE",
            state.refresh_rate().to_string(),
        ));
    }

    if state.block_telemetry() {
        environment.push((telemetry::BLOCK_HOST_VARIABLE, telemetry::blocked_hosts()));
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

    if let Some(vr_session) = vr_session {
        if vr_session.runs_on_zink() {
            environment.retain(|(name, _)| name != VULKAN_DRIVERS_VARIABLE);
        }
        environment.extend(vr_session.environment());
    }

    environment
}

fn studio_arguments(state: &AppState, target: &StudioTarget) -> Vec<OsString> {
    match target {
        StudioTarget::StartPage => Vec::new(),
        StudioTarget::Link(link) => vec![PROTOCOL_STRING_ARGUMENT.into(), link.into()],
        StudioTarget::PlaceFile(place_file) => vec![
            "-task".into(),
            "EditFile".into(),
            "-localPlaceFile".into(),
            prefix::path_seen_from_macos(
                prefix::directory_for(&state.prefix()).as_deref(),
                place_file,
            )
            .into(),
        ],
    }
}

fn vr_log_line(vr: &Result<VrSession, String>) -> String {
    match vr {
        Ok(vr_session) => vr_session.description(),
        Err(reason) => format!("VR: {reason} Roblox starts without VR."),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_the_wrapper_like_a_shell() {
        assert_eq!(
            launch_wrapper_words("gamemoderun gamescope -f -W 1920 --").unwrap(),
            ["gamemoderun", "gamescope", "-f", "-W", "1920", "--"]
        );
        assert_eq!(
            launch_wrapper_words("\"/opt/my tools/run\" 'a b'").unwrap(),
            ["/opt/my tools/run", "a b"]
        );
        assert!(launch_wrapper_words("").unwrap().is_empty());
    }

    #[test]
    fn reports_an_unclosed_quote() {
        assert!(launch_wrapper_words("gamescope \"-f").is_err());
        assert!(launch_wrapper_problem("gamescope \"-f").is_some());
    }

    #[test]
    fn an_empty_wrapper_has_no_problem() {
        assert_eq!(launch_wrapper_problem(""), None);
        assert_eq!(launch_wrapper_problem("   "), None);
    }
}
