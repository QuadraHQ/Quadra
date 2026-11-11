mod account;
mod background;
mod cpu_limit;
mod desktop_entries;
mod destination;
mod diagnostics;
mod discord_ipc;
mod discord_presence;
mod download;
mod fast_flag_allowlist;
mod fast_flags;
mod game_activity;
mod game_details;
mod handlers;
mod launch;
mod launch_request;
mod live_log;
mod mangohud;
mod mime_defaults;
mod mods;
mod paths;
mod prefix;
mod quadra_rosemary;
mod regions;
mod roblox;
mod runners;
mod server_history;
mod server_location;
mod telemetry;
mod updater;
mod vr;

pub use desktop_entries::Shortcut;
pub use handlers::Handler;
pub use launch_request::{LaunchRequest, StudioTarget};
pub use mods::{disabled_units as disabled_mods, enable_unit as enable_mod, folder as mods_folder};
pub use quadra_rosemary::VERSION as QUADRA_ROSEMARY_VERSION;

use crate::state::{AppState, Client, QUADRA_PREFIX, QUADRA_RUNNER, Regions};
use crate::utils::is_executable_file;
use background::{run_quietly, run_task};
use destination::Destination;
use gtk::glib;
use gtk::prelude::*;
use runners::Runner;
use std::path::Path;

const PROPERTIES_THAT_DECIDE_SELECTED_RUNNER: [&str; 2] = ["runner", "custom-runner-path"];

pub fn initialize(state: &AppState) {
    for property in PROPERTIES_THAT_DECIDE_SELECTED_RUNNER {
        state.connect_notify_local(Some(property), |state, _| {
            refresh_selected_runner_status(state)
        });
    }

    state.connect_notify_local(Some("prefix"), |state, _| account::refresh(state));
    updater::initialize(state);
    fast_flag_allowlist::initialize();
    discord_presence::initialize(state);
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

    state.set_darwin_base_installed(prefix::has_darwin_base(&paths::quadra_prefix_directory()));
    account::refresh(state);
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

    run_task(state, quadra_rosemary::install, |state, ()| {
        refresh(state);
        state.set_runner(QUADRA_RUNNER);
    });
}

pub fn install_darwin_base(state: &AppState) {
    run_prefix_task(
        state,
        "Installing the Darwin base",
        prefix::install_darwin_base,
    );
}

pub fn repair_prefix(state: &AppState) {
    run_prefix_task(state, "Repairing the prefix", prefix::repair);
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
    let rosemary = runner.executable.clone();

    run_task(
        state,
        move |progress| {
            if should_install_quadra_rosemary {
                quadra_rosemary::install(progress)?;
            }

            roblox::install_or_update(Client::Player, installed.as_ref(), progress)?;
            progress.step("Applying mods");
            mods::prepare(&mod_settings)?;
            let vr_session = vr_settings.map(|settings| vr::prepare(settings, &rosemary));
            Ok((destination.resolve(progress), vr_session))
        },
        move |state, (launch, vr_session)| {
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

    run_task(
        state,
        move |progress| {
            if should_install_quadra_rosemary {
                quadra_rosemary::install(progress)?;
            }
            roblox::install_or_update(Client::Studio, installed.as_ref(), progress)
        },
        move |state, ()| {
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

pub fn handler_is_enabled(handler: Handler) -> bool {
    handlers::is_enabled(handler)
}

pub fn set_handler_enabled(state: &AppState, handler: Handler, enabled: bool) {
    let result = if enabled {
        handlers::enable(handler)
    } else {
        handlers::disable(handler)
    };
    if let Err(error) = result {
        state.report_error(&format!("{error:#}"));
    }
}

pub fn mangohud_is_installed() -> bool {
    mangohud::is_installed()
}

pub fn stop(state: &AppState) {
    if state.running() && !state.stopping() {
        state.set_stopping(true);
        launch::stop_roblox();
    }
}

pub fn use_custom_runner(state: &AppState, executable: &Path) {
    if !is_executable_file(executable) {
        state.report_error(&format!(
            "{} isn't an executable file, so it can't be used as a runner.",
            executable.display()
        ));
        return;
    }

    state.set_custom_runner_path(executable.display().to_string());
    refresh(state);
    state.set_runner(runners::custom_runner_name(executable));
}

fn refresh_selected_runner_status(state: &AppState) {
    let selected_runner_is_installed =
        selected_runner(state).is_some_and(|runner| runner.is_installed());
    state.set_rosemary_installed(selected_runner_is_installed);

    let installed_rosemary = selected_runner(state).filter(Runner::is_installed);
    state.set_vr_status(vr::status(
        installed_rosemary
            .as_ref()
            .map(|runner| runner.executable.as_path()),
    ));
}

fn prefer_an_installed_runner(state: &AppState) {
    let quadra_rosemary_is_missing = !quadra_rosemary::executable().is_file();
    let other_installed_runner = runners::discover(&state.custom_runner_path())
        .into_iter()
        .find(|runner| !runner.is_quadra() && runner.is_installed());

    if quadra_rosemary_is_missing && let Some(runner) = other_installed_runner {
        state.set_runner(runner.name);
    }
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

fn selected_runner(state: &AppState) -> Option<Runner> {
    runners::find(&state.runner(), &state.custom_runner_path())
}

fn run_prefix_task(
    state: &AppState,
    description: &'static str,
    action: fn(&Path, &Path) -> anyhow::Result<()>,
) {
    if state.busy() {
        return;
    }
    let Some(runner) = selected_runner(state).filter(Runner::is_installed) else {
        state.report_error("The selected runner isn't installed. Install it first.");
        return;
    };

    run_task(
        state,
        move |progress| {
            progress.step(description);
            action(&runner.executable, &paths::quadra_prefix_directory())
        },
        |state, ()| refresh(state),
    );
}
