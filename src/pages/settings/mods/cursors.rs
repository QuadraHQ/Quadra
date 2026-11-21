use crate::backend;
use crate::components::file_row::{ChosenPath, Pick, file_row};
use crate::components::picker::{Card, Opener, choosing, picker_card, picker_grid, picker_row};
use crate::components::preferences::{preference_group, searchable};
use crate::components::style::Look;
use crate::quadra::Message;
use crate::state::{AppState, Choice, CursorStyle, Picker};
use iced::gradient::Linear;
use iced::widget::{container, image, text};
use iced::{Background, Color, Degrees, Element, Gradient, Length, color};
use std::sync::LazyLock;

const CURSOR_FILES: &[Pick] = &[Pick::File(&["png"]), Pick::Folder];
const CURSORS_PER_ROW: usize = 4;
const PREVIEW_HEIGHT: f32 = 64.0;
const SHOWN_SCALE: f32 = 0.75;
const SKY_TOP: Color = color!(0x6fa8dc);
const SKY_BOTTOM: Color = color!(0x8fbf6a);

pub fn cursors_group<'a>(state: &AppState, look: Look) -> Element<'a, Message> {
    let style = state.cursor_style();
    let custom_row = (style == CursorStyle::Custom).then(|| {
        file_row(
            state,
            look,
            ChosenPath::Cursor,
            "Custom cursor",
            CURSOR_FILES,
        )
    });

    preference_group(look)
        .title("Cursors")
        .push(picker_row(
            state,
            look,
            Opener {
                picker: Picker::Cursor,
                title: "Cursor style",
                subtitle: "Replaces the mouse cursors inside the game",
                shown: text(style.label())
                    .font(look.font)
                    .size(look.sized(14.0))
                    .into(),
            },
        ))
        .push_maybe(
            (state.open_picker() == Picker::Cursor)
                .then(|| searchable(cursor_picker(style, look), "Cursor style picker preview")),
        )
        .push_maybe(custom_row)
        .into()
}

fn cursor_picker<'a>(chosen: CursorStyle, look: Look) -> Element<'a, Message> {
    picker_grid(
        CursorStyle::ALL.iter().map(|&style| {
            picker_card(
                look,
                Card {
                    preview: cursor_preview(style, look),
                    preview_height: PREVIEW_HEIGHT,
                    preview_background: Background::Gradient(Gradient::Linear(
                        Linear::new(Degrees(180.0))
                            .add_stop(0.0, SKY_TOP)
                            .add_stop(1.0, SKY_BOTTOM),
                    )),
                    label: style.label().to_owned(),
                    is_chosen: style == chosen,
                    on_press: choosing(move |state| state.set_cursor_style(style)),
                    trailing: None,
                },
            )
        }),
        CURSORS_PER_ROW,
    )
}

struct Arrow {
    style: CursorStyle,
    handle: image::Handle,
    size: f32,
}

fn arrow_of(style: CursorStyle) -> Option<(image::Handle, f32)> {
    static ARROWS: LazyLock<Vec<Arrow>> = LazyLock::new(|| {
        CursorStyle::ALL
            .iter()
            .filter_map(|&style| {
                let bytes = backend::cursor_arrow(style)?;
                let (width, _) = ::image::ImageReader::new(std::io::Cursor::new(bytes))
                    .with_guessed_format()
                    .ok()?
                    .into_dimensions()
                    .ok()?;
                Some(Arrow {
                    style,
                    handle: image::Handle::from_bytes(bytes),
                    size: width as f32 * SHOWN_SCALE,
                })
            })
            .collect()
    });
    ARROWS
        .iter()
        .find(|arrow| arrow.style == style)
        .map(|arrow| (arrow.handle.clone(), arrow.size))
}

fn cursor_preview<'a>(style: CursorStyle, look: Look) -> Element<'a, Message> {
    let shown: Element<'a, Message> = match arrow_of(style) {
        Some((handle, size)) => image(handle).width(size).height(size).into(),
        None => text(match style {
            CursorStyle::Custom => "Your file",
            _ => "Roblox's own",
        })
        .font(look.font)
        .size(look.sized(12.0))
        .color(Color::WHITE)
        .into(),
    };
    container(shown)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}
