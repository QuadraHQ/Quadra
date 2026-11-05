use crate::backend;
use crate::components::error_dialog::show_errors_from;
use crate::components::header::bar;
use crate::components::launch_window::launch_window;
use crate::components::play_action::PlayAction;
use crate::components::tray::{TrayIcon, TrayRequest};
use crate::pages::home::home_page;
use crate::pages::settings::settings_page;
use crate::state::{AppState, LaunchMode};
use adw::{Application, ApplicationWindow, ToolbarView, ViewStack, prelude::*};
use gtk::glib;
use std::cell::RefCell;
use std::rc::Rc;

const PROPERTIES_THAT_DECIDE_STAYING_OPEN: [&str; 4] =
    ["busy", "running", "run-in-background", "launch-mode"];
const PROPERTIES_THAT_DECIDE_TRAY_ICON: [&str; 2] = ["run-in-background", "launch-mode"];

#[derive(Clone, Copy)]
enum Page {
    Home,
    Settings,
}

impl Page {
    fn name(self) -> &'static str {
        match self {
            Self::Home => "home",
            Self::Settings => "settings",
        }
    }
}

pub struct Quadra {
    app: Application,
    state: AppState,
    window: ApplicationWindow,
    launch_window: ApplicationWindow,
    pages: ViewStack,
    tray_icon: RefCell<Option<TrayIcon>>,
}

impl Quadra {
    pub fn start(app: &Application, state: &AppState) -> Rc<Self> {
        let pages = pages(state);
        let window = window(app, state, &pages);
        let quadra = Rc::new(Self {
            app: app.clone(),
            state: state.clone(),
            window,
            launch_window: launch_window(app, state),
            pages,
            tray_icon: RefCell::new(None),
        });

        quadra.window.connect_visible_notify(glib::clone!(
            #[weak]
            quadra,
            move |_| quadra.quit_if_nothing_keeps_it_open()
        ));
        for property in PROPERTIES_THAT_DECIDE_STAYING_OPEN {
            state.connect_notify_local(
                Some(property),
                glib::clone!(
                    #[weak]
                    quadra,
                    move |_, _| quadra.quit_if_nothing_keeps_it_open()
                ),
            );
        }
        for property in PROPERTIES_THAT_DECIDE_TRAY_ICON {
            state.connect_notify_local(
                Some(property),
                glib::clone!(
                    #[weak]
                    quadra,
                    move |_, _| quadra.show_tray_icon_when_wanted()
                ),
            );
        }
        quadra.show_tray_icon_when_wanted();

        quadra
    }

    pub fn activate(&self) {
        let should_launch_roblox_right_away = self.state.launch_mode() == LaunchMode::LaunchRoblox
            && !self.state.running()
            && !self.state.busy();

        if should_launch_roblox_right_away {
            backend::play(&self.state);
            if self.state.busy() {
                self.launch_window.present();
            }
        } else {
            self.show(Page::Home);
        }
    }

    fn show(&self, page: Page) {
        self.pages.set_visible_child_name(page.name());
        self.window.present();
    }

    fn handle(&self, request: TrayRequest) {
        match request {
            TrayRequest::OpenHome => self.show(Page::Home),
            TrayRequest::OpenSettings => self.show(Page::Settings),
            TrayRequest::PerformPlayAction => {
                let action = PlayAction::for_state(&self.state);
                if action.is_available(&self.state) {
                    action.perform(&self.state);
                }
            }
            TrayRequest::Quit => self.app.quit(),
        }
    }

    fn wants_tray_icon(&self) -> bool {
        self.state.run_in_background() || self.state.launch_mode() == LaunchMode::LaunchRoblox
    }

    fn should_keep_running_without_window(&self) -> bool {
        let can_be_reopened_from_tray = self.tray_icon.borrow().is_some();
        let is_launching_roblox_right_away = self.state.launch_mode() == LaunchMode::LaunchRoblox
            && (self.state.busy() || self.state.running());

        (self.state.run_in_background() && can_be_reopened_from_tray)
            || is_launching_roblox_right_away
    }

    fn quit_if_nothing_keeps_it_open(&self) {
        if !self.window.is_visible() && !self.should_keep_running_without_window() {
            self.app.quit();
        }
    }

    fn show_tray_icon_when_wanted(self: &Rc<Self>) {
        let wants_tray_icon = self.wants_tray_icon();
        let has_tray_icon = self.tray_icon.borrow().is_some();

        if wants_tray_icon && !has_tray_icon {
            let quadra = Rc::downgrade(self);
            let tray_icon = TrayIcon::show(&self.state, move |request| {
                if let Some(quadra) = quadra.upgrade() {
                    quadra.handle(request);
                }
            });
            match tray_icon {
                Ok(tray_icon) => *self.tray_icon.borrow_mut() = Some(tray_icon),
                Err(error) => eprintln!("Failed to show the tray icon: {error}"),
            }
        } else if !wants_tray_icon && has_tray_icon {
            self.tray_icon.borrow_mut().take();
        }
    }
}

fn pages(state: &AppState) -> ViewStack {
    let pages = ViewStack::new();

    pages.add_titled_with_icon(
        &home_page(state),
        Some(Page::Home.name()),
        "Home",
        "go-home-symbolic",
    );
    pages.add_titled_with_icon(
        &settings_page(state),
        Some(Page::Settings.name()),
        "Settings",
        "emblem-system-symbolic",
    );

    pages
}

fn window(app: &Application, state: &AppState, pages: &ViewStack) -> ApplicationWindow {
    let toolbar_view = ToolbarView::new();
    toolbar_view.add_top_bar(&bar(pages));
    toolbar_view.set_content(Some(pages));

    let window = ApplicationWindow::builder()
        .application(app)
        .title("Quadra")
        .default_width(800)
        .default_height(600)
        .hide_on_close(true)
        .content(&toolbar_view)
        .build();

    state.connect_error_message_notify(glib::clone!(
        #[weak]
        window,
        move |_| window.present()
    ));
    show_errors_from(state, &window);

    window
}
