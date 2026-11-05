mod components;
mod pages;
mod state;
mod utils;

use crate::components::header::bar;
use crate::pages::home::home_page;
use crate::pages::settings::settings_page;
use state::AppState;

use adw::{Application, ApplicationWindow, ToolbarView, ViewStack, prelude::*};
use gtk::glib;

const APP_ID: &str = "org.quadrahq.Quadra1";

fn main() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();
    let state = AppState::load();

    app.connect_activate(glib::clone!(
        #[strong]
        state,
        move |app| build_ui(app, &state)
    ));

    app.connect_shutdown(move |_| state.save_pending_changes());

    app.connect_startup(|_| {
        let provider = gtk::CssProvider::new();
        provider.load_from_string(
            ".app-header { padding-left: 0px; }
             .app-header .app-icon { margin-left: 8px; }",
        );
        gtk::style_context_add_provider_for_display(
            &gtk::gdk::Display::default().unwrap(),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    });

    app.run()
}

fn build_ui(app: &Application, state: &AppState) {
    let stack = ViewStack::new();

    stack.add_titled_with_icon(&home_page(state), Some("home"), "Home", "go-home-symbolic");
    stack.add_titled_with_icon(
        &settings_page(state),
        Some("settings"),
        "Settings",
        "emblem-system-symbolic",
    );

    let header_bar = bar(&stack);
    let toolbar_view = ToolbarView::new();

    toolbar_view.add_top_bar(&header_bar);
    toolbar_view.set_content(Some(&stack));

    let window = ApplicationWindow::builder()
        .application(app)
        .title("Quadra")
        .default_width(800)
        .default_height(600)
        .content(&toolbar_view)
        .build();

    window.present();
}
