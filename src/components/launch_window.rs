use super::icon::app_icon;
use super::roblox_status::{PROPERTIES_THAT_DECIDE_STATUS, status_text};
use crate::state::{AppState, Client};
use adw::prelude::*;
use gtk::glib;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

const PULSE_EVERY: Duration = Duration::from_millis(100);

const PROPERTIES_THAT_DECIDE_LAUNCH_FINISHED: [&str; 3] =
    ["busy", "running", "roblox-graphics-started"];

fn launch_finished(state: &AppState) -> bool {
    let roblox_is_open_or_gone = state.roblox_graphics_started() || !state.running();
    !state.busy() && roblox_is_open_or_gone
}

fn has_measurable_progress(state: &AppState) -> bool {
    state.task_progress() > 0.0
}

#[derive(Clone)]
struct LaunchProgress {
    status: gtk::Label,
    progress_bar: gtk::ProgressBar,
    pulsing: Rc<RefCell<Option<glib::SourceId>>>,
}

impl LaunchProgress {
    fn show(&self, state: &AppState) {
        self.status.set_label(&status_text(state, Client::Player));
        if has_measurable_progress(state) {
            self.progress_bar.set_fraction(state.task_progress());
        }
    }

    fn start_pulsing(&self, state: &AppState) {
        let progress_bar = self.progress_bar.downgrade();
        let state = state.downgrade();
        let pulsing = glib::timeout_add_local(PULSE_EVERY, move || {
            let (Some(progress_bar), Some(state)) = (progress_bar.upgrade(), state.upgrade())
            else {
                return glib::ControlFlow::Break;
            };
            if !has_measurable_progress(&state) {
                progress_bar.pulse();
            }
            glib::ControlFlow::Continue
        });
        self.stop_pulsing();
        self.pulsing.replace(Some(pulsing));
    }

    fn stop_pulsing(&self) {
        if let Some(pulsing) = self.pulsing.take() {
            pulsing.remove();
        }
    }
}

pub fn launch_window(app: &adw::Application, state: &AppState) -> adw::ApplicationWindow {
    let title = gtk::Label::builder()
        .label("Starting Roblox")
        .css_classes(["title-2"])
        .build();

    let progress = LaunchProgress {
        status: gtk::Label::builder()
            .css_classes(["dimmed"])
            .wrap(true)
            .justify(gtk::Justification::Center)
            .build(),
        progress_bar: gtk::ProgressBar::new(),
        pulsing: Rc::default(),
    };

    progress.progress_bar.connect_map(glib::clone!(
        #[strong]
        progress,
        #[weak]
        state,
        move |_| progress.start_pulsing(&state)
    ));
    progress.progress_bar.connect_unmap(glib::clone!(
        #[strong]
        progress,
        move |_| progress.stop_pulsing()
    ));

    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .margin_top(32)
        .margin_start(32)
        .margin_end(32)
        .margin_bottom(32)
        .build();
    content.append(&app_icon(64));
    content.append(&title);
    content.append(&progress.status);
    content.append(&progress.progress_bar);

    let toolbar_view = adw::ToolbarView::new();
    toolbar_view.set_content(Some(&content));

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Starting Roblox")
        .default_width(360)
        .resizable(false)
        .hide_on_close(true)
        .content(&toolbar_view)
        .build();

    progress.show(state);
    for property in PROPERTIES_THAT_DECIDE_STATUS {
        state.connect_notify_local(
            Some(property),
            glib::clone!(
                #[strong]
                progress,
                move |state, _| progress.show(state)
            ),
        );
    }

    for property in PROPERTIES_THAT_DECIDE_LAUNCH_FINISHED {
        state.connect_notify_local(
            Some(property),
            glib::clone!(
                #[weak]
                window,
                move |state, _| {
                    if launch_finished(state) {
                        window.set_visible(false);
                    }
                }
            ),
        );
    }

    window
}
