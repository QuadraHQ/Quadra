use crate::backend::{self, LaunchRequest};
use crate::command_line::{self, Invocation};
use crate::components::close_roblox_dialog::confirm_closing_roblox;
use crate::components::dialog::{self, Alert, Dialog};
use crate::components::error_dialog::error_dialog;
use crate::components::file_dialog;
use crate::components::file_row::{ChosenPath, Pick};
use crate::components::grid_backdrop::GridBackdrop;
use crate::components::header::bar;
use crate::components::icon::app_icon_rgba;
use crate::components::launch_window::{self, Pulse};
use crate::components::line_field_backdrop::{self, LineField};
use crate::components::logs::LiveLog;
use crate::components::play_action::PlayAction;
use crate::components::style::Look;
use crate::components::title_bar::{self, ButtonLayout, TitleBar};
use crate::components::tray::{TrayIcon, TrayRequest};
use crate::components::update_banner::update_banner;
use crate::main_loop;
use crate::pages::home::home_page;
use crate::pages::servers::{Servers, ServersMessage};
use crate::pages::settings::{self, Settings, SettingsMessage};
use crate::single_instance;
use crate::state::{AppState, Client, LaunchMode, LogChange, Theme};
use crate::utils::{BUNDLED_FONTS, open_with_default_app};
use iced::futures::StreamExt;
use iced::widget::{Column, column, space, stack, text_editor};
use iced::{Element, Size, Subscription, Task, keyboard, system, theme, window};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::fmt;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

const PROPERTIES_THAT_DECIDE_STAYING_OPEN: [&str; 5] = [
    "busy",
    "running",
    "run-in-background",
    "launch-mode",
    "hide-launcher-while-playing",
];
const PROPERTIES_THAT_DECIDE_TRAY_ICON: [&str; 2] = ["run-in-background", "launch-mode"];
const PROPERTIES_THAT_DECIDE_LAUNCH_FINISHED: [&str; 3] =
    ["busy", "running", "roblox-graphics-started"];
const PROPERTIES_THAT_DECIDE_LOOK: [&str; 2] = ["theme", "system-prefers-dark"];
const MAIN_WINDOW_SIZE: Size = Size::new(800.0, 600.0);
const MAIN_WINDOW_MINIMUM_SIZE: Size = Size::new(360.0, 420.0);
const PULSE_EVERY: std::time::Duration = std::time::Duration::from_millis(50);

#[derive(Clone)]
pub struct Act(Arc<dyn Fn(&AppState) + Send + Sync>);

impl fmt::Debug for Act {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Act")
    }
}

pub fn act(change: impl Fn(&AppState) + Send + Sync + 'static) -> Message {
    Message::Act(Act(Arc::new(change)))
}

#[derive(Debug, Clone, Copy)]
pub enum WindowAction {
    Grab,
    Move,
    Release,
    ToggleMaximize,
    Minimize,
    ShowMenu,
    Resize(window::Direction),
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Home,
    Servers,
    Settings,
}

impl Page {
    pub fn title(self) -> &'static str {
        match self {
            Self::Home => "Home",
            Self::Servers => "Servers",
            Self::Settings => "Settings",
        }
    }

    fn is_shown(self, state: &AppState) -> bool {
        self != Self::Servers || state.selected_client() == Client::Player
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    MainLoop,
    Ignored,
    Act(Act),
    Invoked(Invocation),
    SystemTheme(theme::Mode),
    CloseRequested(window::Id),
    WindowClosed(window::Id),
    Window(window::Id, WindowAction),
    WindowResized(window::Id),
    WindowMaximized(window::Id, bool),
    ShowPage(Page),
    Alert(Alert),
    Respond(usize),
    DismissDialog,
    ShowLogs,
    ShowAbout,
    ShowFlagImport,
    ShowFlagExport,
    ImportFlags,
    CopyExportedFlags,
    SaveExportedFlags,
    ExportPathChosen(Option<PathBuf>),
    LogAction(text_editor::Action),
    DialogEditor(text_editor::Action, bool),
    CopyToClipboard(String),
    Open(String),
    PickPath(ChosenPath, Pick),
    PathPicked(ChosenPath, Option<PathBuf>),
    Settings(SettingsMessage),
    Servers(ServersMessage),
    QuitClosingRoblox,
    Pulse,
    LineFieldTick,
}

enum Reaction {
    ErrorReported,
    RunningChanged,
    StayingOpenMayHaveChanged,
    TrayIconMayBeWanted,
    LaunchProgressChanged,
    LookChanged,
    TitleBarChanged,
    ButtonLayoutChanged,
    Log(Option<String>),
    Tray(TrayRequest),
}

type Reactions = Rc<RefCell<VecDeque<Reaction>>>;

pub struct Quadra {
    state: AppState,
    reactions: Reactions,
    main_window: Option<window::Id>,
    launch_window: Option<window::Id>,
    page: Page,
    servers: Servers,
    settings: Settings,
    dialogs: Vec<Dialog>,
    live_log: LiveLog,
    look: Look,
    iced_theme: iced::Theme,
    grid_backdrop: GridBackdrop,
    line_field: Option<LineField>,
    pulse: Pulse,
    tray_icon: Option<TrayIcon>,
    launched_directly: bool,
    quits_once_roblox_exits: bool,
    grabbed_window: Option<window::Id>,
    main_window_is_maximized: bool,
    button_layout: ButtonLayout,
}

pub fn run(invocation: Invocation) -> iced::Result {
    let mut daemon = iced::daemon(
        move || Quadra::start(invocation.clone()),
        Quadra::update,
        Quadra::view,
    )
    .title(Quadra::title)
    .theme(Quadra::theme)
    .style(Quadra::style)
    .subscription(Quadra::subscription);
    for font in BUNDLED_FONTS {
        daemon = daemon.font(font);
    }
    daemon.run()
}

impl Quadra {
    fn start(invocation: Invocation) -> (Self, Task<Message>) {
        let state = AppState::load();
        let reactions = Reactions::default();
        backend::initialize(&state);
        backend::load_window_button_layout(&state);
        settings::keep_choices_valid(&state);
        react_to_state(&state, &reactions);

        let look = Look::of(&state);
        let mut quadra = Self {
            servers: Servers::default(),
            settings: Settings::new(&state),
            state,
            reactions,
            main_window: None,
            launch_window: None,
            page: Page::Home,
            dialogs: Vec::new(),
            live_log: LiveLog::default(),
            iced_theme: look.iced_theme(),
            look,
            grid_backdrop: GridBackdrop::default(),
            line_field: None,
            pulse: Pulse::default(),
            tray_icon: None,
            launched_directly: false,
            quits_once_roblox_exits: false,
            grabbed_window: None,
            main_window_is_maximized: false,
            button_layout: ButtonLayout::parse(""),
        };
        quadra.restyle();
        quadra.show_tray_icon_when_wanted();

        let handled = quadra.handle(invocation);
        let reacted = quadra.react();
        let system_theme = system::theme().map(Message::SystemTheme);
        (quadra, Task::batch([system_theme, handled, reacted]))
    }

    fn handle(&mut self, invocation: Invocation) -> Task<Message> {
        match command_line::launch_requested_by(&invocation) {
            Some(request) => self.launch_directly(request),
            None => self.activate(),
        }
    }

    fn activate(&mut self) -> Task<Message> {
        if self.state.launch_mode() == LaunchMode::LaunchRoblox {
            self.launch_directly(LaunchRequest::Roblox { link: None })
        } else {
            self.show(Page::Home)
        }
    }

    fn launch_directly(&mut self, request: LaunchRequest) -> Task<Message> {
        if self.state.running() || self.state.busy() {
            return self.show(Page::Home);
        }

        self.launched_directly = true;
        self.state.set_selected_client(request.client());
        backend::launch(&self.state, request);
        if self.state.busy() {
            self.open_launch_window()
        } else {
            Task::none()
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        let task = match message {
            Message::MainLoop => {
                main_loop::run_pending();
                Task::none()
            }
            Message::Ignored => Task::none(),
            Message::Act(Act(change)) => {
                change(&self.state);
                Task::none()
            }
            Message::Invoked(invocation) => self.handle(invocation),
            Message::SystemTheme(mode) => {
                let prefers_dark = mode == theme::Mode::Dark;
                if self.state.system_prefers_dark() != prefers_dark {
                    self.state.set_system_prefers_dark(prefers_dark);
                }
                Task::none()
            }
            Message::CloseRequested(id) if Some(id) == self.main_window => self.close_main_window(),
            Message::CloseRequested(id) if Some(id) == self.launch_window => {
                self.close_launch_window()
            }
            Message::CloseRequested(id) => window::close(id),
            Message::Window(id, action) => self.act_on_window(id, action),
            Message::WindowResized(id) => window::is_maximized(id)
                .map(move |maximized| Message::WindowMaximized(id, maximized)),
            Message::WindowMaximized(id, maximized) => {
                if Some(id) == self.main_window {
                    self.main_window_is_maximized = maximized;
                }
                Task::none()
            }
            Message::WindowClosed(id) => {
                if Some(id) == self.main_window {
                    self.main_window = None;
                    self.main_window_is_maximized = false;
                    return Task::batch([self.quit_if_nothing_keeps_it_open(), self.react()]);
                }
                if Some(id) == self.launch_window {
                    self.launch_window = None;
                }
                Task::none()
            }
            Message::ShowPage(page) => self.show(page),
            Message::Alert(alert) => {
                self.dialogs.push(Dialog::Alert(alert));
                Task::none()
            }
            Message::Respond(position) => {
                let response = self
                    .dialogs
                    .pop()
                    .and_then(|dialog| dialog.responses().get(position).cloned())
                    .and_then(|response| response.then);
                match response {
                    Some(message) => self.update(message),
                    None => Task::none(),
                }
            }
            Message::DismissDialog => {
                self.dialogs.pop();
                Task::none()
            }
            Message::ShowLogs => {
                self.dialogs.push(Dialog::Logs);
                Task::none()
            }
            Message::ShowAbout => {
                self.dialogs.push(Dialog::About);
                Task::none()
            }
            Message::ShowFlagImport => {
                self.dialogs
                    .push(Dialog::ImportFlags(text_editor::Content::new()));
                Task::none()
            }
            Message::ShowFlagExport => {
                self.dialogs
                    .push(Dialog::ExportFlags(settings::export_dialog(&self.state)));
                Task::none()
            }
            Message::ImportFlags => {
                if let Some(Dialog::ImportFlags(content)) = self.dialogs.last()
                    && settings::import_flags(&self.state, &content.text())
                {
                    self.dialogs.pop();
                    self.settings.reload_custom_flags(&self.state);
                }
                Task::none()
            }
            Message::CopyExportedFlags => {
                let json = self.state.fast_flags().as_json();
                iced::clipboard::write(json)
            }
            Message::SaveExportedFlags => {
                file_dialog::save_file("Save FastFlags", settings::EXPORT_FILE_NAME)
                    .map(Message::ExportPathChosen)
            }
            Message::ExportPathChosen(chosen) => {
                if let Some(path) = chosen {
                    let json = self.state.fast_flags().as_json();
                    settings::save_exported(&self.state, &path, &json);
                }
                Task::none()
            }
            Message::LogAction(action) => {
                self.live_log.perform(action);
                Task::none()
            }
            Message::DialogEditor(action, editable) => {
                if let Some(Dialog::ImportFlags(content) | Dialog::ExportFlags(content)) =
                    self.dialogs.last_mut()
                    && (editable || !action.is_edit())
                {
                    content.perform(action);
                }
                Task::none()
            }
            Message::CopyToClipboard(text) => iced::clipboard::write(text),
            Message::Open(uri) => {
                open_with_default_app(uri);
                Task::none()
            }
            Message::PickPath(chosen_path, pick) => {
                let picked = match pick {
                    Pick::File(extensions) => file_dialog::pick_file("Choose a file", extensions),
                    Pick::Folder => file_dialog::pick_folder("Choose a folder"),
                };
                picked.map(move |path| Message::PathPicked(chosen_path, path))
            }
            Message::PathPicked(chosen_path, path) => {
                if let Some(path) = path {
                    chosen_path.set(&self.state, path.display().to_string());
                }
                Task::none()
            }
            Message::Settings(message) => self.settings.update(&self.state, message),
            Message::Servers(message) => {
                self.servers.update(&self.state, message);
                Task::none()
            }
            Message::QuitClosingRoblox => self.quit(),
            Message::Pulse => {
                self.pulse.advance();
                Task::none()
            }
            Message::LineFieldTick => {
                if let Some(line_field) = &mut self.line_field {
                    line_field.tick();
                }
                Task::none()
            }
        };
        Task::batch([task, self.react()])
    }

    fn react(&mut self) -> Task<Message> {
        let mut tasks = Vec::new();
        loop {
            let Some(reaction) = self.reactions.borrow_mut().pop_front() else {
                break;
            };
            tasks.push(match reaction {
                Reaction::ErrorReported => {
                    let message = self.state.error_message();
                    self.dialogs.push(Dialog::Alert(error_dialog(&message)));
                    self.present_main_window()
                }
                Reaction::RunningChanged => self.adjust_window_to_running_roblox(),
                Reaction::StayingOpenMayHaveChanged => self.quit_if_nothing_keeps_it_open(),
                Reaction::TrayIconMayBeWanted => {
                    self.show_tray_icon_when_wanted();
                    Task::none()
                }
                Reaction::LaunchProgressChanged => {
                    if launch_window::launch_finished(&self.state) {
                        self.close_launch_window()
                    } else {
                        Task::none()
                    }
                }
                Reaction::LookChanged => {
                    self.restyle();
                    Task::none()
                }
                Reaction::TitleBarChanged => self.redecorate_windows(),
                Reaction::ButtonLayoutChanged => {
                    self.button_layout = ButtonLayout::parse(&self.state.window_button_layout());
                    Task::none()
                }
                Reaction::Log(appended) => {
                    match &appended {
                        Some(text) => self.live_log.follow(LogChange::Appended(text)),
                        None => self.live_log.follow(LogChange::Cleared),
                    }
                    Task::none()
                }
                Reaction::Tray(request) => self.handle_tray(request),
            });
        }
        Task::batch(tasks)
    }

    fn act_on_window(&mut self, id: window::Id, action: WindowAction) -> Task<Message> {
        match action {
            WindowAction::Grab => {
                self.grabbed_window = Some(id);
                Task::none()
            }
            WindowAction::Move => {
                if self.grabbed_window.take() == Some(id) {
                    window::drag(id)
                } else {
                    Task::none()
                }
            }
            WindowAction::Release => {
                self.grabbed_window = None;
                Task::none()
            }
            WindowAction::ToggleMaximize => {
                self.grabbed_window = None;
                window::toggle_maximize(id)
            }
            WindowAction::Minimize => window::minimize(id, true),
            WindowAction::ShowMenu => window::show_system_menu(id),
            WindowAction::Resize(direction) => window::drag_resize(id, direction),
            WindowAction::Close => self.update(Message::CloseRequested(id)),
        }
    }

    fn redecorate_windows(&mut self) -> Task<Message> {
        Task::batch(
            [self.main_window, self.launch_window]
                .into_iter()
                .flatten()
                .map(window::toggle_decorations),
        )
    }

    fn uses_own_title_bar(&self) -> bool {
        !self.state.system_title_bar()
    }

    fn restyle(&mut self) {
        let look = Look::of(&self.state);
        if look != self.look {
            self.look = look;
            self.iced_theme = look.iced_theme();
        }
        let wants_line_field = self.state.theme() == Theme::Monochrome;
        if wants_line_field != self.line_field.is_some() {
            self.line_field = wants_line_field.then(LineField::default);
        }
    }

    fn handle_tray(&mut self, request: TrayRequest) -> Task<Message> {
        match request {
            TrayRequest::OpenHome => self.show(Page::Home),
            TrayRequest::OpenSettings => self.show(Page::Settings),
            TrayRequest::PerformPlayAction => {
                let action = PlayAction::for_client(&self.state, self.state.current_client());
                if action.is_available(&self.state) {
                    action.perform(&self.state);
                }
                Task::none()
            }
            TrayRequest::Quit => self.confirm_quit(),
        }
    }

    fn show(&mut self, page: Page) -> Task<Message> {
        self.launched_directly = false;
        self.page = page;
        if page == Page::Servers {
            self.servers.shown(&self.state);
        }
        self.present_main_window()
    }

    fn present_main_window(&mut self) -> Task<Message> {
        if let Some(id) = self.main_window {
            return window::gain_focus(id);
        }
        let (id, opened) = window::open(window_settings(
            MAIN_WINDOW_SIZE,
            true,
            self.state.system_title_bar(),
        ));
        self.main_window = Some(id);
        opened.discard()
    }

    fn hide_main_window(&mut self) -> Task<Message> {
        let closed = match self.main_window.take() {
            Some(id) => window::close(id),
            None => Task::none(),
        };
        Task::batch([closed, self.quit_if_nothing_keeps_it_open()])
    }

    fn open_launch_window(&mut self) -> Task<Message> {
        if let Some(id) = self.launch_window {
            return window::gain_focus(id);
        }
        let (id, opened) = window::open(window_settings(
            launch_window::SIZE,
            false,
            self.state.system_title_bar(),
        ));
        self.launch_window = Some(id);
        opened.discard()
    }

    fn close_launch_window(&mut self) -> Task<Message> {
        match self.launch_window.take() {
            Some(id) => window::close(id),
            None => Task::none(),
        }
    }

    fn close_main_window(&mut self) -> Task<Message> {
        if self.state.running() && !self.should_keep_running_without_window() {
            self.confirm_quit()
        } else {
            self.hide_main_window()
        }
    }

    fn confirm_quit(&mut self) -> Task<Message> {
        if !self.state.running() {
            return self.exit();
        }
        self.dialogs.push(Dialog::Alert(confirm_closing_roblox()));
        self.present_main_window()
    }

    fn quit(&mut self) -> Task<Message> {
        if !self.state.running() {
            return self.exit();
        }
        self.quits_once_roblox_exits = true;
        let hidden = self.hide_main_window();
        backend::stop(&self.state);
        hidden
    }

    fn exit(&mut self) -> Task<Message> {
        self.state.save_pending_changes();
        self.tray_icon.take();
        iced::exit()
    }

    fn adjust_window_to_running_roblox(&mut self) -> Task<Message> {
        if self.quits_once_roblox_exits && !self.state.running() {
            return self.exit();
        }
        if self.state.running() && self.state.hide_launcher_while_playing() {
            self.hide_main_window()
        } else if !self.state.running() && self.state.open_after_exit() {
            self.show(Page::Home)
        } else {
            Task::none()
        }
    }

    fn wants_tray_icon(&self) -> bool {
        self.state.run_in_background() || self.state.launch_mode() == LaunchMode::LaunchRoblox
    }

    fn should_keep_running_without_window(&self) -> bool {
        let can_be_reopened_from_tray = self.tray_icon.is_some();
        let launches_without_window =
            self.state.launch_mode() == LaunchMode::LaunchRoblox || self.launched_directly;
        let is_launching_right_away =
            launches_without_window && (self.state.busy() || self.state.running());
        let is_hidden_while_playing =
            self.state.hide_launcher_while_playing() && self.state.running();

        (self.state.run_in_background() && can_be_reopened_from_tray)
            || is_launching_right_away
            || is_hidden_while_playing
            || self.quits_once_roblox_exits
    }

    fn quit_if_nothing_keeps_it_open(&mut self) -> Task<Message> {
        if self.main_window.is_none() && !self.should_keep_running_without_window() {
            self.exit()
        } else {
            Task::none()
        }
    }

    fn show_tray_icon_when_wanted(&mut self) {
        let wants_tray_icon = self.wants_tray_icon();
        if wants_tray_icon && self.tray_icon.is_none() {
            let reactions = self.reactions.clone();
            let tray_icon = TrayIcon::show(&self.state, move |request| {
                reactions.borrow_mut().push_back(Reaction::Tray(request));
            });
            match tray_icon {
                Ok(tray_icon) => self.tray_icon = Some(tray_icon),
                Err(error) => eprintln!("Failed to show the tray icon: {error}"),
            }
        } else if !wants_tray_icon {
            self.tray_icon.take();
        }
    }

    fn title(&self, id: window::Id) -> String {
        if Some(id) == self.launch_window {
            launch_window::starting_text(&self.state)
        } else {
            "Quadra".to_owned()
        }
    }

    fn theme(&self, _id: window::Id) -> iced::Theme {
        self.iced_theme.clone()
    }

    fn style(&self, _theme: &iced::Theme) -> theme::Style {
        self.look.app_style()
    }

    fn view(&self, id: window::Id) -> Element<'_, Message> {
        if Some(id) == self.main_window {
            self.main_view()
        } else if Some(id) == self.launch_window {
            self.launch_view(id)
        } else {
            space().into()
        }
    }

    fn main_view(&self) -> Element<'_, Message> {
        let look = self.look;
        let state = &self.state;
        let pages: Vec<Page> = [Page::Home, Page::Servers, Page::Settings]
            .into_iter()
            .filter(|page| page.is_shown(state))
            .collect();
        let page = if self.page.is_shown(state) {
            self.page
        } else {
            Page::Home
        };

        let page_content = match page {
            Page::Home => home_page(state, look),
            Page::Servers => self.servers.view(state, look),
            Page::Settings => self.settings.view(state, look),
        };
        let mut below_title = stack![
            Column::new()
                .push(update_banner(state, look))
                .push(page_content)
        ];
        if let Some(dialog) = self.dialogs.last() {
            below_title = below_title.push(dialog::view(dialog, &self.live_log, look));
        }

        let Some(window) = self.main_window.filter(|_| self.uses_own_title_bar()) else {
            return stack![]
                .push(self.backdrop())
                .push(column![bar(look, page, &pages), below_title])
                .into();
        };

        let title_bar = TitleBar {
            window,
            look,
            shown: page,
            pages,
            layout: &self.button_layout,
            is_maximized: self.main_window_is_maximized,
            is_grabbed: self.grabbed_window == Some(window),
        };
        let mut layers = stack![]
            .push(self.backdrop())
            .push(column![title_bar.view(), below_title]);
        if !self.main_window_is_maximized {
            layers = layers
                .push(title_bar::outline(look))
                .push(title_bar::resize_edges(window));
        }
        layers.into()
    }

    fn backdrop(&self) -> Option<Element<'_, Message>> {
        match self.state.theme() {
            Theme::Grid => Some(self.grid_backdrop.view()),
            Theme::Monochrome => self.line_field.as_ref().map(LineField::view),
            Theme::Adwaita => None,
        }
    }

    fn launch_view(&self, window: window::Id) -> Element<'_, Message> {
        let content = launch_window::view(&self.state, self.pulse);
        if !self.uses_own_title_bar() {
            return content;
        }
        stack![
            title_bar::grab_area(window, self.grabbed_window == Some(window), content),
            title_bar::close_button(window, self.look),
            title_bar::outline(self.look),
        ]
        .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        let mut subscriptions = vec![
            Subscription::run(|| main_loop::wakeups().map(|()| Message::MainLoop)),
            Subscription::run(|| single_instance::forwarded_invocations().map(Message::Invoked)),
            window::close_requests().map(Message::CloseRequested),
            window::close_events().map(Message::WindowClosed),
            system::theme_changes().map(Message::SystemTheme),
        ];

        if self.uses_own_title_bar() {
            subscriptions.push(window::resize_events().map(|(id, _)| Message::WindowResized(id)));
        }

        if !self.dialogs.is_empty() {
            subscriptions.push(keyboard::listen().filter_map(|event| match event {
                keyboard::Event::KeyPressed {
                    key: keyboard::Key::Named(keyboard::key::Named::Escape),
                    ..
                } => Some(Message::DismissDialog),
                _ => None,
            }));
        }

        let is_pulsing =
            self.launch_window.is_some() && !launch_window::has_measurable_progress(&self.state);
        if is_pulsing {
            subscriptions.push(
                Subscription::run_with(PULSE_EVERY, main_loop::ticks).map(|()| Message::Pulse),
            );
        }

        let is_drawing_line_field = self.main_window.is_some()
            && self
                .line_field
                .as_ref()
                .is_some_and(|line_field| !line_field.is_drawn());
        if is_drawing_line_field {
            subscriptions.push(
                Subscription::run_with(line_field_backdrop::TICK_INTERVAL, main_loop::ticks)
                    .map(|()| Message::LineFieldTick),
            );
        }

        Subscription::batch(subscriptions)
    }
}

fn react_to_state(state: &AppState, reactions: &Reactions) {
    let push = |reaction: fn() -> Reaction| {
        let reactions = reactions.clone();
        move |_: &AppState| reactions.borrow_mut().push_back(reaction())
    };

    state.connect_property_notify("error-message", push(|| Reaction::ErrorReported));
    state.connect_property_notify("running", push(|| Reaction::RunningChanged));
    state.connect_properties_notify(
        &PROPERTIES_THAT_DECIDE_STAYING_OPEN,
        push(|| Reaction::StayingOpenMayHaveChanged),
    );
    state.connect_properties_notify(
        &PROPERTIES_THAT_DECIDE_TRAY_ICON,
        push(|| Reaction::TrayIconMayBeWanted),
    );
    state.connect_properties_notify(
        &PROPERTIES_THAT_DECIDE_LAUNCH_FINISHED,
        push(|| Reaction::LaunchProgressChanged),
    );
    state.connect_properties_notify(&PROPERTIES_THAT_DECIDE_LOOK, push(|| Reaction::LookChanged));
    state.connect_property_notify("system-title-bar", push(|| Reaction::TitleBarChanged));
    state.connect_property_notify(
        "window-button-layout",
        push(|| Reaction::ButtonLayoutChanged),
    );

    let reactions = reactions.clone();
    state.connect_log_changed(move |_, change| {
        let appended = match change {
            LogChange::Appended(text) => Some(text.to_owned()),
            LogChange::Cleared => None,
        };
        reactions.borrow_mut().push_back(Reaction::Log(appended));
    });
}

fn window_settings(size: Size, resizable: bool, decorations: bool) -> window::Settings {
    window::Settings {
        size,
        min_size: resizable.then_some(MAIN_WINDOW_MINIMUM_SIZE),
        resizable,
        decorations,
        exit_on_close_request: false,
        icon: app_icon_rgba()
            .and_then(|(rgba, width, height)| window::icon::from_rgba(rgba, width, height).ok()),
        platform_specific: window::settings::PlatformSpecific {
            application_id: crate::APP_ID.to_owned(),
            ..Default::default()
        },
        ..window::Settings::default()
    }
}
