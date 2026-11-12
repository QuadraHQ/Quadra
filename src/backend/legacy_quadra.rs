mod leftovers;
mod running;

use super::background::run_task;
use crate::state::AppState;
use crate::utils::is_running_in_flatpak;
use anyhow::ensure;
use gtk::glib;
use leftovers::{Roots, find, remove};
use std::path::Path;

const PROCESSES: &str = "/proc";

pub fn look_for(state: &AppState) {
    if is_running_in_flatpak() || state.legacy_cleanup_dismissed() {
        return;
    }

    let (total_sender, total_receiver) = async_channel::bounded(1);
    std::thread::spawn(move || {
        let total: u64 = find(&Roots::of_user())
            .iter()
            .map(|leftover| leftover.bytes)
            .sum();
        let _ = total_sender.send_blocking(total);
    });

    let state = state.clone();
    glib::spawn_future_local(async move {
        if let Ok(total) = total_receiver.recv().await {
            state.set_legacy_cleanup_bytes(total);
        }
    });
}

pub fn delete(state: &AppState) {
    if state.busy() {
        return;
    }

    run_task(
        state,
        |progress| {
            ensure!(
                !running::old_quadra_is_running(Path::new(PROCESSES)),
                "Close the old Quadra first, then try again."
            );
            progress.step("Removing the old Quadra");
            let errors = remove(&find(&Roots::of_user()));
            ensure!(
                errors.is_empty(),
                "Couldn't remove everything:\n{}",
                errors.join("\n")
            );
            Ok(())
        },
        |state, ()| state.set_legacy_cleanup_bytes(0),
    );
}
