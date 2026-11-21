use super::content_files::ContentFiles;
use super::custom_font;
use super::settings::{ModSettings, RobloxTheme};
use crate::state::CursorStyle;
use crate::utils::BUNDLED_FONTS;
use std::io;
use std::path::Path;
use tiny_skia::{
    Color, FillRule, LineCap, LineJoin, Paint, PathBuilder, Pixmap, Stroke, Transform,
};

const CURSOR_DIRECTORY: &str = "textures/Cursors/KeyboardMouse";
const ARROW_CURSORS: [&str; 2] = ["ArrowCursor.png", "ArrowFarCursor.png"];
const LOCKED_CURSOR: &str = "textures/MouseLockedCursor.png";
const CURSOR_CANVAS: u32 = 64;
const LOCKED_CANVAS: u32 = 32;
const ARROW_OUTLINE: [(f32, f32); 7] = [
    (0.0, 0.0),
    (0.0, 17.0),
    (4.2, 13.2),
    (7.0, 19.6),
    (9.6, 18.5),
    (6.9, 12.3),
    (12.4, 12.3),
];
const ARROW_SCALE: f32 = 1.25;
const ARROW_MARGIN: f32 = 1.5;

pub fn apply(files: &mut ContentFiles, settings: &ModSettings) -> io::Result<()> {
    let Some(theme) = &settings.theme else {
        return Ok(());
    };
    if settings.cursor_style == CursorStyle::Stock {
        let arrow =
            arrow_cursor(theme).ok_or_else(|| io::Error::other("couldn't draw the cursor"))?;
        for name in ARROW_CURSORS {
            files.place_bytes(&Path::new(CURSOR_DIRECTORY).join(name), &arrow)?;
        }
        let locked =
            locked_cursor(theme).ok_or_else(|| io::Error::other("couldn't draw the cursor"))?;
        files.place_bytes(Path::new(LOCKED_CURSOR), &locked)?;
    }
    if settings.custom_font.as_os_str().is_empty()
        && let Some(family) = &theme.font
        && let Some((font, extension)) = font_named(family)
    {
        custom_font::apply_bytes(files, &font, extension)?;
    }
    Ok(())
}

fn paint(color: [f32; 4]) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.set_color(
        Color::from_rgba(color[0], color[1], color[2], color[3]).unwrap_or(Color::WHITE),
    );
    paint.anti_alias = true;
    paint
}

fn rounded_stroke(width: f32) -> Stroke {
    Stroke {
        width,
        line_join: LineJoin::Round,
        line_cap: LineCap::Round,
        ..Stroke::default()
    }
}

pub fn arrow_cursor(theme: &RobloxTheme) -> Option<Vec<u8>> {
    let mut pixmap = Pixmap::new(CURSOR_CANVAS, CURSOR_CANVAS)?;
    let mut builder = PathBuilder::new();
    for (index, &(x, y)) in ARROW_OUTLINE.iter().enumerate() {
        let (x, y) = (
            ARROW_MARGIN + x * ARROW_SCALE,
            ARROW_MARGIN + y * ARROW_SCALE,
        );
        if index == 0 {
            builder.move_to(x, y);
        } else {
            builder.line_to(x, y);
        }
    }
    builder.close();
    let arrow = builder.finish()?;
    pixmap.stroke_path(
        &arrow,
        &paint(theme.outline),
        &rounded_stroke(2.6),
        Transform::identity(),
        None,
    );
    pixmap.fill_path(
        &arrow,
        &paint(theme.accent),
        FillRule::Winding,
        Transform::identity(),
        None,
    );
    pixmap.encode_png().ok()
}

pub fn locked_cursor(theme: &RobloxTheme) -> Option<Vec<u8>> {
    let mut pixmap = Pixmap::new(LOCKED_CANVAS, LOCKED_CANVAS)?;
    let middle = LOCKED_CANVAS as f32 / 2.0;
    let ring = PathBuilder::from_circle(middle, middle, 9.0)?;
    let dot = PathBuilder::from_circle(middle, middle, 2.2)?;
    pixmap.stroke_path(
        &ring,
        &paint(theme.outline),
        &rounded_stroke(4.6),
        Transform::identity(),
        None,
    );
    pixmap.stroke_path(
        &ring,
        &paint(theme.accent),
        &rounded_stroke(2.4),
        Transform::identity(),
        None,
    );
    pixmap.stroke_path(
        &dot,
        &paint(theme.outline),
        &rounded_stroke(2.0),
        Transform::identity(),
        None,
    );
    pixmap.fill_path(
        &dot,
        &paint(theme.accent),
        FillRule::Winding,
        Transform::identity(),
        None,
    );
    pixmap.encode_png().ok()
}

fn font_named(family: &str) -> Option<(Vec<u8>, &'static str)> {
    match family {
        "Archivo" => return Some((BUNDLED_FONTS[0].to_vec(), "ttf")),
        "JetBrains Mono" => return Some((BUNDLED_FONTS[1].to_vec(), "ttf")),
        _ => {}
    }
    let mut fonts = fontdb::Database::new();
    fonts.load_system_fonts();
    let query = fontdb::Query {
        families: &[fontdb::Family::Name(family)],
        ..fontdb::Query::default()
    };
    let id = fonts.query(&query)?;
    let (source, _) = fonts.face_source(id)?;
    let fontdb::Source::File(path) = source else {
        return None;
    };
    let extension = match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "ttf" => "ttf",
        "otf" => "otf",
        _ => return None,
    };
    Some((std::fs::read(path).ok()?, extension))
}

#[cfg(test)]
mod tests {
    use super::{arrow_cursor, locked_cursor};
    use crate::backend::mods::settings::RobloxTheme;

    #[test]
    fn draws_cursors_as_png() {
        let theme = RobloxTheme {
            accent: [1.0, 0.43, 0.75, 1.0],
            outline: [0.05, 0.04, 0.11, 0.9],
            font: None,
        };
        for image in [arrow_cursor(&theme), locked_cursor(&theme)] {
            assert!(image.is_some_and(|png| png.starts_with(b"\x89PNG")));
        }
    }
}
