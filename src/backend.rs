mod background;
mod download;
mod launch;
mod live_log;
mod paths;
mod prefix;
mod quadra_rosemary;
mod roblox;
mod runners;

pub use quadra_rosemary::VERSION as QUADRA_ROSEMARY_VERSION;

use crate::state::{AppState, QUADRA_PREFIX, QUADRA_RUNNER};
use crate::utils::is_executable_file;
use background::run_task;
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

    refresh(state);
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

    let installed_roblox = roblox::installed_release();
    state.set_roblox_installed(installed_roblox.is_some());
    state.set_roblox_version(
        installed_roblox
            .map(|release| release.version)
            .unwrap_or_default(),
    );

    state.set_darwin_base_installed(prefix::has_darwin_base(&paths::quadra_prefix_directory()));
}

pub fn install_roblox(state: &AppState) {
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
    let roblox_is_missing = !state.roblox_installed();

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

            if roblox_is_missing {
                progress.step("Finding the latest Roblox version");
                let latest = roblox::latest_release()?;
                roblox::install(&latest, progress)?;
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
    if state.busy() || state.running() {
        return;
    }
    let Some(runner) = selected_runner(state) else {
        state.report_error("The selected runner can't be found. Choose another one in Settings.");
        return;
    };
    let should_install_quadra_rosemary = runner.is_quadra() && !runner.is_installed();
    let installed = roblox::installed_release();

    run_task(
        state,
        move |progress| {
            if should_install_quadra_rosemary {
                quadra_rosemary::install(progress)?;
            }

            progress.step("Checking for Roblox updates");
            match roblox::latest_release() {
                Ok(latest) if Some(&latest) != installed.as_ref() => {
                    roblox::install(&latest, progress)?
                }
                Ok(_) => {}
                Err(error) if installed.is_some() => {
                    eprintln!("Starting the installed Roblox without updating: {error:#}")
                }
                Err(error) => return Err(error),
            }
            Ok(())
        },
        move |state, ()| {
            refresh(state);
            if let Err(error) = launch::start_roblox(state, &runner) {
                state.report_error(&format!("{error:#}"));
            }
        },
    );
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
