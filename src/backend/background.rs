use crate::state::AppState;
use anyhow::anyhow;
use gtk::glib;
use std::cell::Cell;

enum ProgressUpdate {
    Step(String),
    Fraction(f64),
}

pub struct Progress {
    updates: async_channel::Sender<ProgressUpdate>,
    last_reported_percent: Cell<Option<u32>>,
}

impl Progress {
    pub fn step(&self, description: impl Into<String>) {
        self.last_reported_percent.set(None);
        let _ = self
            .updates
            .send_blocking(ProgressUpdate::Step(description.into()));
    }

    pub fn fraction(&self, fraction: f64) {
        let percent = (fraction * 100.0) as u32;
        if self.last_reported_percent.replace(Some(percent)) != Some(percent) {
            let _ = self
                .updates
                .send_blocking(ProgressUpdate::Fraction(fraction));
        }
    }
}

pub fn run_task<T: Send + 'static>(
    state: &AppState,
    work: impl FnOnce(&Progress) -> anyhow::Result<T> + Send + 'static,
    when_finished: impl FnOnce(&AppState, T) + 'static,
) {
    let (update_sender, update_receiver) = async_channel::unbounded();
    let (result_sender, result_receiver) = async_channel::bounded(1);

    std::thread::spawn(move || {
        let progress = Progress {
            updates: update_sender,
            last_reported_percent: Cell::new(None),
        };
        let result = work(&progress);
        let _ = result_sender.send_blocking(result);
    });

    state.set_busy(true);
    let state = state.clone();
    glib::spawn_future_local(async move {
        while let Ok(update) = update_receiver.recv().await {
            match update {
                ProgressUpdate::Step(description) => {
                    state.set_task_description(description);
                    state.set_task_progress(0.0);
                }
                ProgressUpdate::Fraction(fraction) => state.set_task_progress(fraction),
            }
        }

        let result = result_receiver
            .recv()
            .await
            .unwrap_or_else(|_| Err(anyhow!("The background task stopped unexpectedly.")));

        state.set_task_description("");
        state.set_task_progress(0.0);

        match result {
            Ok(value) => when_finished(&state, value),
            Err(error) => state.report_error(&format!("{error:#}")),
        }

        state.set_busy(false);
    });
}
