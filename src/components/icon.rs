use super::style::Look;
use crate::quadra::Message;
use crate::utils::APP_ICON_PNG;
use iced::widget::{image, svg};
use iced::{Element, Length};
use std::sync::LazyLock;

static APP_ICON: LazyLock<image::Handle> =
    LazyLock::new(|| image::Handle::from_bytes(APP_ICON_PNG));

pub fn app_icon<'a>(size: f32) -> Element<'a, Message> {
    image(APP_ICON.clone())
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .into()
}

pub fn app_icon_rgba() -> Option<(Vec<u8>, u32, u32)> {
    match ::image::load_from_memory_with_format(APP_ICON_PNG, ::image::ImageFormat::Png) {
        Ok(icon) => {
            let icon = icon.into_rgba8();
            let (width, height) = icon.dimensions();
            Some((icon.into_raw(), width, height))
        }
        Err(error) => {
            eprintln!("Failed to load the app icon: {error}");
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Symbol {
    Home,
    Server,
    Settings,
    Play,
    Stop,
    Download,
    Terminal,
    Folder,
    File,
    Trash,
    Add,
    Remove,
    Clear,
    Warning,
    Check,
    Update,
    Expand,
    Collapse,
    WindowMinimize,
    WindowMaximize,
    WindowRestore,
    WindowClose,
}

macro_rules! symbols {
    ($($symbol:ident => $paths:literal,)*) => {
        fn handle(symbol: Symbol) -> svg::Handle {
            $(
                #[allow(non_upper_case_globals)]
                static $symbol: LazyLock<svg::Handle> = LazyLock::new(|| {
                    svg::Handle::from_memory(
                        concat!(
                            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">"#,
                            $paths,
                            "</svg>"
                        )
                        .as_bytes(),
                    )
                });
            )*
            match symbol {
                $(Symbol::$symbol => $symbol.clone(),)*
            }
        }
    };
}

symbols! {
    Home => r#"<path d="M2.5 7.5 8 2.5l5.5 5"/><path d="M4 6.5v7h3v-4h2v4h3v-7"/>"#,
    Server => r#"<rect x="2.5" y="2.5" width="11" height="4.5" rx="1"/><rect x="2.5" y="9" width="11" height="4.5" rx="1"/><path d="M5 4.75h.01M5 11.25h.01"/>"#,
    Settings => r#"<circle cx="8" cy="8" r="2.2"/><path d="M8 1.5v2M8 12.5v2M1.5 8h2M12.5 8h2M3.4 3.4l1.4 1.4M11.2 11.2l1.4 1.4M3.4 12.6l1.4-1.4M11.2 4.8l1.4-1.4"/>"#,
    Play => r#"<path d="M4.5 2.75v10.5L13 8z" fill="currentColor"/>"#,
    Stop => r#"<rect x="3.5" y="3.5" width="9" height="9" rx="1" fill="currentColor"/>"#,
    Download => r#"<path d="M8 2v8M4.5 6.75 8 10.25l3.5-3.5M2.5 13.5h11"/>"#,
    Terminal => r#"<rect x="1.5" y="2.5" width="13" height="11" rx="1.5"/><path d="m4.5 6 2 2-2 2M8 10.5h3.5"/>"#,
    Folder => r#"<path d="M1.75 4.25a1 1 0 0 1 1-1h3.5l1.5 1.5h5.5a1 1 0 0 1 1 1v6.5a1 1 0 0 1-1 1H2.75a1 1 0 0 1-1-1z"/>"#,
    File => r#"<path d="M4 1.75h5l3.25 3.25v9.25H4z"/><path d="M9 1.75V5h3.25"/>"#,
    Trash => r#"<path d="M2.5 4h11M6.25 4V2.5h3.5V4M4 4l.75 9.5h6.5L12 4M6.75 6.5v4.5M9.25 6.5v4.5"/>"#,
    Add => r#"<path d="M8 2.5v11M2.5 8h11"/>"#,
    Remove => r#"<path d="M2.5 8h11"/>"#,
    Clear => r#"<path d="m3.5 3.5 9 9M12.5 3.5l-9 9"/>"#,
    Warning => r#"<path d="M8 1.75 14.75 13.75H1.25z"/><path d="M8 6v3.5M8 11.75h.01"/>"#,
    Check => r#"<path d="m2.75 8.5 3.5 3.5 7-7.5"/>"#,
    Update => r#"<circle cx="8" cy="8" r="6.25"/><path d="M8 11V5M5.5 7.5 8 5l2.5 2.5"/>"#,
    Expand => r#"<path d="m4 6 4 4 4-4"/>"#,
    Collapse => r#"<path d="m4 10 4-4 4 4"/>"#,
    WindowMinimize => r#"<path d="M4.5 11.5h7"/>"#,
    WindowMaximize => r#"<rect x="4.5" y="4.5" width="7" height="7" rx="1"/>"#,
    WindowRestore => r#"<rect x="4" y="6" width="6" height="6" rx="1"/><path d="M6.5 6V4.5a.5.5 0 0 1 .5-.5h4.5a.5.5 0 0 1 .5.5V9a.5.5 0 0 1-.5.5H10"/>"#,
    WindowClose => r#"<path d="m5 5 6 6M11 5l-6 6"/>"#,
}

pub fn symbol<'a>(symbol: Symbol, look: Look) -> Element<'a, Message> {
    tinted(symbol, look.text)
}

pub fn tinted<'a>(symbol: Symbol, color: iced::Color) -> Element<'a, Message> {
    sized(symbol, color, 16.0)
}

pub fn sized<'a>(symbol: Symbol, color: iced::Color, size: f32) -> Element<'a, Message> {
    svg(handle(symbol))
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .style(move |_, _| svg::Style { color: Some(color) })
        .into()
}
