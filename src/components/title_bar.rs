use super::header::tabs;
use super::icon::{self, Symbol};
use super::style::{self, Look};
use crate::quadra::{Message, Page, WindowAction};
use iced::mouse::Interaction;
use iced::widget::{
    Row, button, column, container, mouse_area, responsive, row, rule, space, stack,
};
use iced::window::{self, Direction};
use iced::{Alignment, Element, Event, Length, Size, Subscription, event};
use std::sync::Mutex;

pub const HEIGHT: f32 = 46.0;
const COMPACT_BELOW: f32 = 560.0;
const EDGE: f32 = 5.0;
const CORNER: f32 = 12.0;
const BUTTON_SPACING: f32 = 6.0;
const DEFAULT_LAYOUT: &str = ":minimize,maximize,close";
const SIZE_JUMP: f32 = 64.0;

const FILLED_SHARE: f32 = 0.8;

static LAST_SIZES: Mutex<Vec<(window::Id, Size)>> = Mutex::new(Vec::new());
static MONITOR_SIZES: Mutex<Vec<(window::Id, Size)>> = Mutex::new(Vec::new());

pub fn remember_monitor(window: window::Id, monitor: Size) {
    if let Ok(mut monitors) = MONITOR_SIZES.lock() {
        monitors.retain(|(known, _)| *known != window);
        monitors.push((window, monitor));
    }
}

fn monitor_of(window: window::Id) -> Option<Size> {
    let monitors = MONITOR_SIZES.lock().ok()?;
    monitors
        .iter()
        .find(|(known, _)| *known == window)
        .map(|&(_, monitor)| monitor)
}

fn fills(size: Size, monitor: Size) -> bool {
    size.width >= monitor.width * FILLED_SHARE && size.height >= monitor.height * FILLED_SHARE
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowButton {
    Minimize,
    Maximize,
    Close,
}

impl WindowButton {
    fn named(name: &str) -> Option<Self> {
        match name.trim() {
            "minimize" => Some(Self::Minimize),
            "maximize" => Some(Self::Maximize),
            "close" => Some(Self::Close),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ButtonLayout {
    start: Vec<WindowButton>,
    end: Vec<WindowButton>,
}

impl ButtonLayout {
    pub fn parse(layout: &str) -> Self {
        let layout = if layout.trim().is_empty() {
            DEFAULT_LAYOUT
        } else {
            layout
        };
        let (start, end) = layout.split_once(':').unwrap_or((layout, ""));
        let mut seen = Vec::new();
        let mut buttons = |side: &str| -> Vec<WindowButton> {
            side.split(',')
                .filter_map(WindowButton::named)
                .filter(|button| {
                    let is_new = !seen.contains(button);
                    seen.push(*button);
                    is_new
                })
                .collect()
        };
        Self {
            start: buttons(start),
            end: buttons(end),
        }
    }
}

pub struct TitleBar<'a> {
    pub window: window::Id,
    pub look: Look,
    pub shown: Page,
    pub pages: Vec<Page>,
    pub layout: &'a ButtonLayout,
    pub is_maximized: bool,
    pub is_grabbed: bool,
}

impl<'a> TitleBar<'a> {
    pub fn view(self) -> Element<'a, Message> {
        let Self {
            window,
            look,
            shown,
            pages,
            layout,
            is_maximized,
            is_grabbed,
        } = self;

        let mut grip = mouse_area(
            container(space())
                .width(Length::Fill)
                .height(HEIGHT)
                .style(style::bar(look)),
        )
        .on_press(Message::Window(window, WindowAction::Grab))
        .on_release(Message::Window(window, WindowAction::Release))
        .on_double_click(Message::Window(window, WindowAction::ToggleMaximize))
        .on_right_press(Message::Window(window, WindowAction::ShowMenu));
        if is_grabbed {
            grip = grip.on_move(move |_| Message::Window(window, WindowAction::Move));
        }

        let side = |buttons: &[WindowButton]| -> Row<'a, Message> {
            row(buttons
                .iter()
                .map(|&button| window_button(window, button, is_maximized, look)))
            .spacing(BUTTON_SPACING)
            .align_y(Alignment::Center)
        };
        let controls = row![
            side(&layout.start),
            space().width(Length::Fill),
            side(&layout.end)
        ]
        .padding([0, 10])
        .height(HEIGHT)
        .align_y(Alignment::Center);

        let centered_tabs = responsive(move |size| {
            container(tabs(look, shown, &pages, size.width < COMPACT_BELOW))
                .center(Length::Fill)
                .into()
        })
        .height(HEIGHT);

        column![
            stack![grip, controls, centered_tabs],
            rule::horizontal(1).style(style::separator(look)),
        ]
        .into()
    }
}

fn window_button<'a>(
    window: window::Id,
    window_button: WindowButton,
    is_maximized: bool,
    look: Look,
) -> Element<'a, Message> {
    let (symbol, action) = match window_button {
        WindowButton::Minimize => (Symbol::WindowMinimize, WindowAction::Minimize),
        WindowButton::Maximize if is_maximized => {
            (Symbol::WindowRestore, WindowAction::ToggleMaximize)
        }
        WindowButton::Maximize => (Symbol::WindowMaximize, WindowAction::ToggleMaximize),
        WindowButton::Close => (Symbol::WindowClose, WindowAction::Close),
    };
    button(icon::symbol(symbol, look))
        .padding(4)
        .style(style::window_button(look))
        .on_press(Message::Window(window, action))
        .into()
}

pub fn grab_area<'a>(
    window: window::Id,
    is_grabbed: bool,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    let mut area = mouse_area(content)
        .on_press(Message::Window(window, WindowAction::Grab))
        .on_release(Message::Window(window, WindowAction::Release));
    if is_grabbed {
        area = area.on_move(move |_| Message::Window(window, WindowAction::Move));
    }
    area.into()
}

pub fn close_button<'a>(window: window::Id, look: Look) -> Element<'a, Message> {
    container(window_button(window, WindowButton::Close, false, look))
        .padding(10)
        .align_right(Length::Fill)
        .into()
}

pub fn resize_edges<'a>(window: window::Id) -> Element<'a, Message> {
    let handle = |direction: Direction, cursor: Interaction, width: Length, height: Length| {
        mouse_area(space().width(width).height(height))
            .interaction(cursor)
            .on_press(Message::Window(window, WindowAction::Resize(direction)))
    };
    let corner = Length::Fixed(CORNER);
    let edge = Length::Fixed(EDGE);

    column![
        row![
            handle(
                Direction::NorthWest,
                Interaction::ResizingDiagonallyDown,
                corner,
                corner
            ),
            handle(
                Direction::North,
                Interaction::ResizingVertically,
                Length::Fill,
                edge
            ),
            handle(
                Direction::NorthEast,
                Interaction::ResizingDiagonallyUp,
                corner,
                corner
            ),
        ]
        .align_y(Alignment::Start),
        row![
            handle(
                Direction::West,
                Interaction::ResizingHorizontally,
                edge,
                Length::Fill
            ),
            space().width(Length::Fill).height(Length::Fill),
            handle(
                Direction::East,
                Interaction::ResizingHorizontally,
                edge,
                Length::Fill
            ),
        ]
        .height(Length::Fill),
        row![
            handle(
                Direction::SouthWest,
                Interaction::ResizingDiagonallyUp,
                corner,
                corner
            ),
            handle(
                Direction::South,
                Interaction::ResizingVertically,
                Length::Fill,
                edge
            ),
            handle(
                Direction::SouthEast,
                Interaction::ResizingDiagonallyDown,
                corner,
                corner
            ),
        ]
        .align_y(Alignment::End),
    ]
    .into()
}

pub fn size_jumps() -> Subscription<window::Id> {
    event::listen_with(|event, _status, window| match event {
        Event::Window(window::Event::Resized(size)) => jumped(window, size).then_some(window),
        Event::Window(window::Event::Closed) => {
            forget(window);
            None
        }
        _ => None,
    })
}

fn jumped(window: window::Id, size: Size) -> bool {
    let Ok(mut last_sizes) = LAST_SIZES.lock() else {
        return true;
    };
    let previous = match last_sizes.iter_mut().find(|(known, _)| *known == window) {
        Some((_, last_size)) => Some(std::mem::replace(last_size, size)),
        None => {
            last_sizes.push((window, size));
            None
        }
    };
    drop(last_sizes);
    let Some(previous) = previous else {
        return true;
    };
    let jumped_far = (previous.width - size.width).abs() > SIZE_JUMP
        || (previous.height - size.height).abs() > SIZE_JUMP;
    jumped_far
        && monitor_of(window).is_none_or(|monitor| fills(size, monitor) || fills(previous, monitor))
}

fn forget(window: window::Id) {
    for sizes in [&LAST_SIZES, &MONITOR_SIZES] {
        if let Ok(mut sizes) = sizes.lock() {
            sizes.retain(|(known, _)| *known != window);
        }
    }
}

pub fn outline<'a>(look: Look) -> Element<'a, Message> {
    let strip = |width: Length, height: Length| {
        container(space().width(width).height(height)).style(style::outline(look))
    };
    let line = Length::Fixed(1.0);
    column![
        strip(Length::Fill, line),
        row![
            strip(line, Length::Fill),
            space().width(Length::Fill),
            strip(line, Length::Fill)
        ]
        .height(Length::Fill),
        strip(Length::Fill, line),
    ]
    .into()
}

#[cfg(test)]
mod tests {

    #[test]
    fn only_maximise_and_restore_sized_jumps_are_reported() {
        let window = super::window::Id::unique();
        super::remember_monitor(window, super::Size::new(1920.0, 1080.0));
        assert!(super::jumped(window, super::Size::new(800.0, 600.0)));
        assert!(!super::jumped(window, super::Size::new(960.0, 1040.0)));
        assert!(!super::jumped(window, super::Size::new(805.0, 600.0)));
        assert!(super::jumped(window, super::Size::new(1920.0, 1040.0)));
        assert!(super::jumped(window, super::Size::new(800.0, 600.0)));
        super::forget(window);
    }
    use super::*;

    #[test]
    fn reads_gnome_button_layouts() {
        assert_eq!(
            ButtonLayout::parse("appmenu:close"),
            ButtonLayout {
                start: vec![],
                end: vec![WindowButton::Close]
            }
        );
        assert_eq!(
            ButtonLayout::parse("close,minimize,maximize:appmenu"),
            ButtonLayout {
                start: vec![
                    WindowButton::Close,
                    WindowButton::Minimize,
                    WindowButton::Maximize
                ],
                end: vec![]
            }
        );
    }

    #[test]
    fn falls_back_to_the_usual_layout() {
        assert_eq!(
            ButtonLayout::parse(""),
            ButtonLayout {
                start: vec![],
                end: vec![
                    WindowButton::Minimize,
                    WindowButton::Maximize,
                    WindowButton::Close
                ]
            }
        );
    }

    #[test]
    fn ignores_unknown_and_repeated_buttons() {
        assert_eq!(
            ButtonLayout::parse("icon,close:spacer,close,maximize"),
            ButtonLayout {
                start: vec![WindowButton::Close],
                end: vec![WindowButton::Maximize]
            }
        );
    }
}
