mod appimage;
mod install_kind;
mod release;
pub mod version;

use super::background::{run_quietly, run_task};
use crate::state::AppState;
use install_kind::InstallKind;
use release::AppImageUpdate;
use std::cell::{Cell, RefCell};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const VARIABLES_THE_APPIMAGE_RUNTIME_SETS: [&str; 4] = ["APPIMAGE", "APPDIR", "ARGV0", "OWD"];

thread_local! {
    static INSTALL_KIND: InstallKind = InstallKind::detect();
    static AVAILABLE_UPDATE: RefCell<Option<AppImageUpdate>> = const { RefCell::new(None) };
    static CHECKING: Cell<bool> = const { Cell::new(false) };
}

pub fn initialize(state: &AppState) {
    match INSTALL_KIND.with(Clone::clone) {
        InstallKind::AppImage(_) => check(state),
        InstallKind::Flatpak => {
            state.set_update_status("Updates come from Flatpak, not from Quadra itself");
        }
        InstallKind::Other => {
            state.set_update_status("Updates come from wherever you installed Quadra");
        }
    }
}

pub fn can_update_itself() -> bool {
    matches!(INSTALL_KIND.with(Clone::clone), InstallKind::AppImage(_))
}

pub fn running_appimage() -> Option<PathBuf> {
    match INSTALL_KIND.with(Clone::clone) {
        InstallKind::AppImage(appimage) => Some(appimage),
        InstallKind::Flatpak | InstallKind::Other => None,
    }
}

pub fn check(state: &AppState) {
    if !can_update_itself() || CHECKING.replace(true) {
        return;
    }
    state.set_update_status("Checking for updates…");

    let state = state.clone();
    run_quietly(
        || release::newest_after(CURRENT_VERSION),
        move |newest| {
            match newest {
                Ok(Some(update)) => {
                    state.set_update_status(format!("Version {} is available", update.version));
                    state.set_available_update(update.version.clone());
                    AVAILABLE_UPDATE.replace(Some(update));
                }
                Ok(None) => {
                    state.set_update_status("Quadra is up to date");
                    state.set_available_update("");
                    AVAILABLE_UPDATE.replace(None);
                }
                Err(error) => {
                    eprintln!("{error:#}");
                    state.set_update_status("Couldn't check for updates");
                }
            }
            CHECKING.set(false);
        },
    );
}

pub fn install(state: &AppState) {
    if state.busy() || state.running() {
        state.report_error("Close Roblox and wait for Quadra to finish before updating.");
        return;
    }
    let InstallKind::AppImage(running_appimage) = INSTALL_KIND.with(Clone::clone) else {
        return;
    };
    let Some(update) = AVAILABLE_UPDATE.with_borrow(Clone::clone) else {
        return;
    };

    let appimage_to_restart = running_appimage.clone();
    run_task(
        state,
        move |progress| appimage::replace(&update, &running_appimage, progress),
        move |state, ()| restart(state, &appimage_to_restart),
    );
}

fn restart(state: &AppState, appimage: &Path) {
    state.save_pending_changes();
    let mut command = Command::new(appimage);
    for variable in VARIABLES_THE_APPIMAGE_RUNTIME_SETS {
        command.env_remove(variable);
    }
    let error = command.exec();
    state.report_error(&format!(
        "Quadra updated, but couldn't restart itself ({error}). Open it again to use the new version."
    ));
}
