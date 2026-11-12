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
use state::{AppState, Theme};
use std::cell::OnceCell;
use std::rc::Rc;

const APP_ID: &str = "org.quadrahq.Quadra";

fn main() -> glib::ExitCode {
    if let Some(exit_code) = command_line::run_report_if_requested() {
        return exit_code;
    }

    let app = Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();
    command_line::add_options(&app);
    let quadra: Rc<OnceCell<Rc<Quadra>>> = Rc::default();

    app.connect_startup(glib::clone!(
        #[strong]
        quadra,
        move |app| {
            let state = AppState::load();
            backend::initialize(&state);
            follow_theme(&state);
            app.connect_shutdown(glib::clone!(
                #[strong]
                state,
                move |_| state.save_pending_changes()
            ));
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

    app.run()
}

fn follow_theme(state: &AppState) {
    utils::register_bundled_fonts();

    let display = gtk::gdk::Display::default().unwrap();
    let theme = gtk::CssProvider::new();
    gtk::style_context_add_provider_for_display(
        &display,
        &theme,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    let header = gtk::CssProvider::new();
    header.load_from_string(
        ".app-header { padding-left: 0px; }
         .app-header .app-icon { margin-left: 8px; }",
    );
    gtk::style_context_add_provider_for_display(
        &display,
        &header,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    state.follow_properties(&["theme"], move |state| apply_theme(&theme, state.theme()));
}

fn apply_theme(provider: &gtk::CssProvider, theme: Theme) {
    let (color_scheme, stylesheet) = match theme {
        Theme::Grid => (adw::ColorScheme::ForceDark, include_str!("styles/grid.css")),
        Theme::Monochrome => (
            adw::ColorScheme::ForceDark,
            include_str!("styles/monochrome.css"),
        ),
        Theme::Adwaita => (adw::ColorScheme::Default, ""),
    };
    adw::StyleManager::default().set_color_scheme(color_scheme);
    provider.load_from_string(stylesheet);
}
