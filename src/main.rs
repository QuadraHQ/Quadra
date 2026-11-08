mod backend;
mod command_line;
mod components;
mod pages;
mod quadra;
mod state;
mod utils;

use adw::{Application, prelude::*};
use gtk::{gio, glib};
use quadra::Quadra;
use state::AppState;
use std::cell::OnceCell;
use std::rc::Rc;

const APP_ID: &str = "org.quadrahq.Quadra";

fn main() -> glib::ExitCode {
    let app = Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();
    command_line::add_options(&app);
    let state = AppState::load();
    backend::initialize(&state);
    let quadra: Rc<OnceCell<Rc<Quadra>>> = Rc::default();

    app.connect_startup(glib::clone!(
        #[strong]
        state,
        #[strong]
        quadra,
        move |app| {
            load_style();
            quadra.get_or_init(|| Quadra::start(app, &state));
        }
    ));

    app.connect_command_line(glib::clone!(
        #[strong]
        quadra,
        move |_, command_line| {
            if let Some(quadra) = quadra.get() {
                match command_line::launch_requested_by(command_line) {
                    Some(request) => quadra.launch_directly(request),
                    None => quadra.activate(),
                }
            }
            glib::ExitCode::SUCCESS
        }
    ));

    app.connect_activate(move |_| {
        if let Some(quadra) = quadra.get() {
            quadra.activate();
        }
    });

    app.connect_shutdown(move |_| state.save_pending_changes());

    app.run()
}

fn load_style() {
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
}
