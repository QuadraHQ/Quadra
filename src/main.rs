mod components;

use crate::components::header::bar;

use adw::{Application, ApplicationWindow, ToolbarView, ViewStack, prelude::*};
use gtk::glib;

const APP_ID: &str = "org.quadrahq.Quadra";

fn main() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(build_ui);

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

fn build_ui(app: &Application) {
    let stack = ViewStack::new();

    stack.add_titled_with_icon(
        &gtk::Label::new(Some("Home page")),
        Some("home"),
        "Home",
        "go-home-symbolic",
    );
    stack.add_titled_with_icon(
        &gtk::Label::new(Some("Settings page")),
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
        .default_width(960)
        .default_height(800)
        .content(&toolbar_view)
        .build();

    window.present();
}
