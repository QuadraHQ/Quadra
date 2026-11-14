mod account;
mod background;
mod butter_access;
mod compositor_binding;
mod cpu_limit;
mod current_game;
mod desktop_entries;
mod destination;
mod diagnostics;
mod discord_access;
mod discord_ipc;
mod discord_presence;
mod download;
mod fast_flag_allowlist;
mod fast_flags;
mod flatpak_override;
mod game_activity;
mod game_details;
mod handlers;
mod launch;
mod launch_request;
mod live_log;
mod mangohud;
mod mime_defaults;
mod mods;
mod overlay_data;
mod paths;
mod prefix;
mod quadra_rosemary;
mod regions;
mod release_channel;
mod req_agent;
mod roblox;
mod runners;
mod server_history;
mod server_location;
mod telemetry;
mod transport_patch;
mod updater;
mod vr;

pub use butter_access::missing as missing_butter_access;
pub use compositor_binding::{CompositorBinding, compositor_binding_for};
pub use desktop_entries::Shortcut;
pub use discord_access::{MissingDiscordAccess, missing as missing_discord_access};
pub use handlers::Handler;
pub use launch::launch_wrapper_problem;
pub use launch_request::{LaunchRequest, StudioTarget};
pub use mods::{disabled_units as disabled_mods, enable_unit as enable_mod, folder as mods_folder};
pub use quadra_rosemary::VERSION as QUADRA_ROSEMARY_VERSION;
pub use overlay_data::{read_layout as read_overlay_layout, read_notes as read_overlay_notes, write_layout as write_overlay_layout, write_notes as write_overlay_notes};

use crate::state::{AppState, Client, QUADRA_PREFIX, QUADRA_RUNNER, Regions};
use crate::utils::is_executable_file;
use background::{Progress, run_quietly, run_task};
use destination::Destination;
use gtk::glib;
use gtk::prelude::*;
use runners::Runner;
use std::path::{Path, PathBuf};

const PROPERTIES_THAT_DECIDE_SELECTED_RUNNER: [&str; 2] = ["runner", "custom-runner-path"];

pub fn initialize(state: &AppState) {
    state.connect_properties_notify(
        &PROPERTIES_THAT_DECIDE_SELECTED_RUNNER,
        refresh_selected_runner_status,
    );
    state.connect_notify_local(Some("prefix"), |state, _| account::refresh(state));
    updater::initialize(state);
    quadra_rosemary::look_for_update(state);
    fast_flag_allowlist::initialize();
    fast_flags::initialize(state);
    discord_presence::initialize(state);
    current_game::initialize(state);
    server_history::initialize(state);
    server_location::initialize(state);
    refresh(state);
    refresh_desktop_entries();
    if state.first_launch() {
        prefer_an_installed_runner(state);
    }
}

pub fn refresh(state: &AppState) {
    let runners = runners::discover(&state.custom_runner_path());
    state.set_available_runners(
        runners
            .iter()
            .map(|runner| runner.name.clone())
            .collect::<Vec<_>>(),
    );
    state.set_darwin_base_installed(prefix::has_darwin_base(&paths::quadra_prefix_directory()));
    quadra_rosemary::forget_installed_update(state);
    refresh_selected_runner_status(state);

    let installed_roblox = roblox::installed_release(Client::Player);
    state.set_roblox_installed(installed_roblox.is_some());
    state.set_roblox_version(
        installed_roblox
            .map(|release| release.version)
            .unwrap_or_default(),
    );

    let installed_studio = roblox::installed_release(Client::Studio);
    state.set_studio_installed(installed_studio.is_some());
    state.set_studio_version(
        installed_studio
            .map(|release| release.version)
            .unwrap_or_default(),
    );

    account::refresh(state);
}

fn look_for_newer_darwin_base(state: &AppState) {
    let runner = selected_runner(state).filter(Runner::is_installed);
    let (Some(runner), Some(prefix_directory)) = (runner, updatable_prefix(state)) else {
        state.set_newer_darwin_base("");
        return;
    };

    let state = state.clone();
    run_quietly(
        move || prefix::newer_darwin_base(&runner.executable, &prefix_directory),
        move |newer_release| match newer_release {
            Ok(newer_release) => state.set_newer_darwin_base(newer_release.unwrap_or_default()),
            Err(error) => eprintln!("Couldn't check for a newer Darwin base: {error:#}"),
        },
    );
}

fn updatable_prefix(state: &AppState) -> Option<PathBuf> {
    (state.prefix() == QUADRA_PREFIX && state.darwin_base_installed())
        .then(paths::quadra_prefix_directory)
}

fn update_darwin_base_before_launch(
    rosemary: &Path,
    prefix_directory: Option<&Path>,
    progress: &Progress,
) {
    let Some(prefix_directory) = prefix_directory else {
        return;
    };
    if let Err(error) = prefix::update_darwin_base_if_outdated(rosemary, prefix_directory, progress)
    {
        eprintln!("Couldn't update the Darwin base, launching with the current one: {error:#}");
    }
}

pub fn install(state: &AppState, client: Client) {
    if state.busy() {
        return;
    }

    let runner = selected_runner(state);
    let should_install_quadra_rosemary = runner
        .as_ref()
        .is_some_and(|runner| runner.is_quadra() && !runner.is_installed());
    let installed_rosemary = runner
        .filter(Runner::is_installed)
        .map(|runner| runner.executable);
    let quadra_prefix_needs_darwin_base =
        state.prefix() == QUADRA_PREFIX && !state.darwin_base_installed();
    let client_is_missing = !state.is_installed(client);

    run_task(
        state,
        move |progress| {
            let rosemary = if should_install_quadra_rosemary {
                quadra_rosemary::install(progress)?;
                Some(quadra_rosemary::executable())
            } else {
                installed_rosemary
            };

            if let Some(rosemary) = rosemary.filter(|_| quadra_prefix_needs_darwin_base) {
                progress.step("Setting up the prefix");
                prefix::install_darwin_base(&rosemary, &paths::quadra_prefix_directory())?;
            }

            if client_is_missing {
                progress.step(format!("Finding the latest {} version", client.name()));
                let latest = roblox::latest_release(client)?;
                roblox::install(client, &latest, progress)?;
            }

            Ok(())
        },
        |state, ()| refresh(state),
    );
}

pub fn install_rosemary(state: &AppState) {
    if state.busy() {
        return;
    }
    run_task(
        state,
        |progress| quadra_rosemary::install(progress),
        |state, ()| refresh(state),
    );
}

pub fn update_rosemary(state: &AppState) {
    if state.busy() {
        return;
    }
    let Some(runner) = selected_runner(state) else {
        state.report_error("The selected runner can't be found. Choose another one in Settings.");
        return;
    };
    if !runner.is_quadra() {
        state.report_error("Only the Quadra runner can be updated from here.");
        return;
    }
    run_task(
        state,
        move |progress| quadra_rosemary::update_before_launch(&runner.executable, progress),
        |state, ()| refresh(state),
    );
}

pub fn prefer_an_installed_runner(state: &AppState) {
    let runners = runners::discover(&state.custom_runner_path());
    for runner in runners {
        if runner.is_installed() {
            state.set_runner(runner.name);
            return;
        }
    }
}

fn selected_runner(state: &AppState) -> Option<Runner> {
    runners::find(&state.runner(), &state.custom_runner_path())
}

fn refresh_selected_runner_status(state: &AppState) {
    let selected_runner_is_installed =
        selected_runner(state).is_some_and(|runner| runner.is_installed());
    state.set_rosemary_installed(selected_runner_is_installed);

    let installed_rosemary = selected_runner(state).filter(Runner::is_installed);

    run_quietly(
        move || {
            let version = installed_rosemary
                .as_ref()
                .map(|runner| runner.executable.clone())
                .and_then(|_| quadra_rosemary::installed_version());
            Ok(version)
        },
        move |_version| {
            // Rosemary version tracking not implemented in state
        },
    );
}

pub fn play(state: &AppState) {
    start_playing(state, Destination::RobloxApp);
}

pub fn launch(state: &AppState, request: LaunchRequest) {
    match request {
        LaunchRequest::Roblox { link: None } => play(state),
        LaunchRequest::Roblox { link: Some(link) } => start_playing(state, Destination::Link(link)),
        LaunchRequest::Studio(target) => open_studio(state, target),
    }
}

pub fn join_game(state: &AppState, place_id: u64) {
    let region_key = state.preferred_region();
    let destination = if region_key.is_empty() {
        Destination::AnyServer { place_id }
    } else {
        Destination::ServerInRegion {
            place_id,
            region_label: state.regions().label_of(&region_key),
            avoided_job_ids: state.server_history().job_ids_in(place_id),
            region_key,
        }
    };
    start_playing(state, destination);
}

pub fn rejoin_server(state: &AppState, place_id: u64, job_id: &str) {
    start_playing(
        state,
        Destination::Server {
            place_id,
            job_id: job_id.to_owned(),
        },
    );
}

pub fn load_regions(state: &AppState) {
    if !state.regions().0.is_empty() {
        return;
    }

    let state = state.clone();
    run_quietly(regions::available, move |regions| match regions {
        Ok(regions) => state.set_regions(Regions(regions)),
        Err(error) => eprintln!("Couldn't get the list of regions: {error:#}"),
    });
}

pub fn flag_is_ignored(name: &str) -> bool {
    fast_flag_allowlist::is_ignored(name)
}

pub fn check_for_updates(state: &AppState) {
    updater::check(state);
}

pub fn install_update(state: &AppState) {
    updater::install(state);
}

pub fn can_update_itself() -> bool {
    updater::can_update_itself()
}

pub fn hardware_report() -> String {
    diagnostics::hardware_report()
}

pub fn sign_out(state: &AppState) {
    account::sign_out(state);
}

pub fn diagnostics_directory() -> std::path::PathBuf {
    glib::user_special_dir(glib::UserDirectory::Documents).unwrap_or_else(glib::home_dir)
}

pub fn collect_diagnostics_in_background(state: &AppState) {
    if state.busy() {
        return;
    }
    let summary = diagnostics::quadra_summary(state);
    let directory = diagnostics_directory();

    run_task(
        state,
        move |progress| {
            progress.step("Collecting diagnostics");
            diagnostics::collect(&summary, &directory)
        },
        |_, archive| {
            gtk::FileLauncher::new(Some(&gtk::gio::File::for_path(archive)))
                .open_containing_folder(gtk::Window::NONE, gtk::gio::Cancellable::NONE, |_| {});
        },
    );
}

pub fn collect_diagnostics(directory: &Path) -> anyhow::Result<std::path::PathBuf> {
    let state = AppState::load();
    diagnostics::collect(&diagnostics::quadra_summary(&state), directory)
}

pub fn forget_server_history(state: &AppState) {
    server_history::forget_all(state);
}

fn start_playing(state: &AppState, destination: Destination) {
    if state.busy() || state.running() {
        return;
    }
    let Some(runner) = selected_runner(state) else {
        state.report_error("The selected runner can't be found. Choose another one in Settings.");
        return;
    };
    let should_install_quadra_rosemary = runner.is_quadra() && !runner.is_installed();
    let installed = roblox::installed_release(Client::Player);
    let vr_settings = vr::VrSettings::wanted_by(state);
    let mod_settings = mods::ModSettings::of(state);
    let prefix_to_update = updatable_prefix(state);

    run_task(
        state,
        move |progress| {
            if should_install_quadra_rosemary {
                quadra_rosemary::install(progress)?;
            }
            let rosemary = runner.executable.clone();
            update_quadra_rosemary_before_launch(&runner, progress);
            update_darwin_base_before_launch(&rosemary, prefix_to_update.as_deref(), progress);
            roblox::install_or_update(Client::Player, installed.as_ref(), progress)?;
            progress.step("Applying mods");
            mods::prepare(&mod_settings)?;
            let vr_session = vr_settings.map(|settings| vr::prepare(settings, &rosemary));
            Ok((destination.resolve(progress), vr_session, runner))
        },
        move |state, (launch, vr_session, runner)| {
            refresh(state);
            if let Some(picked_server) = launch.picked_server {
                server_history::expect_join_to(picked_server);
            }
            if let Err(error) =
                launch::start_roblox(state, &runner, launch.uri.as_deref(), vr_session.as_ref())
            {
                state.report_error(&format!("{error:#}"));
            }
        },
    );
}

pub fn open_studio(state: &AppState, target: StudioTarget) {
    if state.busy() || state.running() {
        return;
    }
    let Some(runner) = selected_runner(state) else {
        state.report_error("The selected runner can't be found. Choose another one in Settings.");
        return;
    };
    let should_install_quadra_rosemary = runner.is_quadra() && !runner.is_installed();
    let installed = roblox::installed_release(Client::Studio);
    let prefix_to_update = updatable_prefix(state);

    run_task(
        state,
        move |progress| {
            if should_install_quadra_rosemary {
                quadra_rosemary::install(progress)?;
            }
            let rosemary = runner.executable.clone();
            update_quadra_rosemary_before_launch(&runner, progress);
            update_darwin_base_before_launch(
                &rosemary,
                prefix_to_update.as_deref(),
                progress,
            );
            roblox::install_or_update(Client::Studio, installed.as_ref(), progress)?;
            Ok(runner)
        },
        move |state, runner| {
            refresh(state);
            if let Err(error) = launch::start_studio(state, &runner, &target) {
                state.report_error(&format!("{error:#}"));
            }
        },
    );
}

pub fn has_shortcut(shortcut: Shortcut) -> bool {
    shortcut.entry().exists()
}

pub fn add_shortcut(state: &AppState, shortcut: Shortcut) {
    if let Err(error) = shortcut.entry().add() {
        state.report_error(&format!("{error:#}"));
    }
}

pub fn remove_shortcut(state: &AppState, shortcut: Shortcut) {
    if let Err(error) = shortcut.entry().remove() {
        state.report_error(&format!("{error:#}"));
    }
}

pub fn refresh_shortcuts(state: &AppState) {
    for shortcut in Shortcut::ALL {
        if let Err(error) = shortcut.entry().refresh() {
            eprintln!("Couldn't refresh {}: {error:#}", shortcut.entry().file_name);
        }
    }
}

fn update_quadra_rosemary_before_launch(runner: &Runner, progress: &Progress) {
    if runner.is_quadra() && runner.is_installed() {
        let rosemary = runner.executable.clone();
        if let Err(error) = quadra_rosemary::update_before_launch(&rosemary, progress) {
            eprintln!("Couldn't update Quadra Rosemary before launch: {error:#}");
        }
    }
}

pub fn start_current_game_status_updater(state: &AppState) {
    if state.running() {
        current_game::initialize(state);
    }
}

pub fn handler_is_enabled(handler: Handler) -> bool {
    handler.entry().exists()
}

pub fn set_handler_enabled(state: &AppState, handler: Handler, enabled: bool) {
    if enabled {
        if let Err(error) = handler.entry().add() {
            state.report_error(&format!("{error:#}"));
        }
    } else if let Err(error) = handler.entry().remove() {
        state.report_error(&format!("{error:#}"));
    }
}

pub fn mangohud_is_installed() -> bool {
    std::process::Command::new("mangohud")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

pub fn stop(state: &AppState) {
    if state.running() && !state.stopping() {
        state.set_stopping(true);
        launch::stop_roblox();
    }
}

pub fn use_custom_runner(state: &AppState, executable: &std::path::Path) {
    state.set_runner("Custom".to_owned());
    state.set_custom_runner_path(executable.display().to_string());
}

pub fn install_darwin_base(state: &AppState) {
    if state.busy() {
        return;
    }
    let Some(runner) = selected_runner(state) else {
        state.report_error("The selected runner can't be found. Choose another one in Settings.");
        return;
    };
    run_task(
        state,
        move |progress| {
            prefix::install_darwin_base(&runner.executable, &paths::quadra_prefix_directory())?;
            Ok(())
        },
        |state, ()| refresh(state),
    );
}

pub fn update_darwin_base(state: &AppState) {
    if state.busy() {
        return;
    }
    let Some(runner) = selected_runner(state) else {
        state.report_error("The selected runner can't be found. Choose another one in Settings.");
        return;
    };
    run_task(
        state,
        move |progress| {
            prefix::update_darwin_base(&runner.executable, &paths::quadra_prefix_directory())?;
            Ok(())
        },
        |state, ()| refresh(state),
    );
}

pub fn repair_prefix(state: &AppState) {
    if state.busy() {
        return;
    }
    let Some(runner) = selected_runner(state) else {
        state.report_error("The selected runner can't be found. Choose another one in Settings.");
        return;
    };
    run_task(
        state,
        move |progress| {
            prefix::repair(&runner.executable, &paths::quadra_prefix_directory())?;
            Ok(())
        },
        |state, ()| refresh(state),
    );
}

pub fn update_quadra_rosemary(state: &AppState) {
    if state.busy() {
        return;
    }
    let Some(runner) = selected_runner(state) else {
        state.report_error("The selected runner can't be found. Choose another one in Settings.");
        return;
    };
    if !runner.is_quadra() {
        state.report_error("Only the Quadra runner can be updated from here.");
        return;
    }
    run_task(
        state,
        move |progress| {
            let rosemary = runner.executable.clone();
            quadra_rosemary::update_before_launch(&rosemary, progress)
        },
        |state, ()| refresh(state),
    );
}

pub fn installed_quadra_rosemary_version() -> Option<String> {
    let runner = runners::find(&QUADRA_RUNNER, &"");
    runner.and_then(|runner| quadra_rosemary::installed_version())
}

pub fn game_link(place_id: u64, job_id: &str) -> String {
    format!("roblox://placeId={place_id}&gameInstanceId={job_id}")
}

pub fn launch_command() -> Vec<String> {
    let runner = std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("quadra-rewrite"));
    vec![runner.to_string_lossy().to_string()]
}

fn refresh_desktop_entries() {
    let entries = Shortcut::ALL
        .map(Shortcut::entry)
        .into_iter()
        .chain(Handler::ALL.map(Handler::entry));
    for entry in entries {
        if let Err(error) = entry.refresh() {
            eprintln!("Couldn't update {}: {error:#}", entry.file_name);
        }
    }
}