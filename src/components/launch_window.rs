use super::icon::app_icon;
use super::roblox_status::{PROPERTIES_THAT_DECIDE_STATUS, status_text};
use crate::state::AppState;
use adw::prelude::*;
use gtk::glib;

#[derive(Clone)]
struct LaunchProgress {
    status: gtk::Label,
    spinner: adw::Spinner,
    progress_bar: gtk::ProgressBar,
}

impl LaunchProgress {
    fn show(&self, state: &AppState) {
        let has_measurable_progress = state.task_progress() > 0.0;

        self.status.set_label(&status_text(state));
        self.spinner.set_visible(!has_measurable_progress);
        self.progress_bar.set_visible(has_measurable_progress);
        self.progress_bar.set_fraction(state.task_progress());
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
        spinner: adw::Spinner::builder()
            .width_request(32)
            .height_request(32)
            .build(),
        progress_bar: gtk::ProgressBar::new(),
    };

    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .margin_start(32)
        .margin_end(32)
        .margin_bottom(32)
        .build();
    content.append(&app_icon(64));
    content.append(&title);
    content.append(&progress.status);
    content.append(&progress.spinner);
    content.append(&progress.progress_bar);

    let toolbar_view = adw::ToolbarView::new();
    toolbar_view.add_top_bar(&adw::HeaderBar::builder().show_title(false).build());
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

    state.connect_busy_notify(glib::clone!(
        #[weak]
        window,
        move |state| {
            if !state.busy() {
                window.set_visible(false);
            }
        }
    ));

    window
}
