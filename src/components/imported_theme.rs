use super::gradient_backdrop::Glow;
use super::style::Look;
use crate::state::BackgroundAnimation;
use iced::{Color, Degrees, Font};

const MOST_GRADIENT_STOPS: usize = 8;
const MOST_ANIMATION_COLORS: usize = 3;

#[derive(Debug, Clone)]
pub struct ImportedTheme {
    pub name: String,
    pub look: Look,
    pub background: Option<Glow>,
    pub animation: BackgroundAnimation,
    pub animation_colors: Vec<Color>,
    pub problems: Vec<String>,
}

#[derive(Default)]
struct Declared {
    name: Option<String>,
    colors: Vec<(&'static str, Color)>,
    radius: Option<f32>,
    font: Option<String>,
    mono_font: Option<String>,
    background: Option<Glow>,
    animation: Option<BackgroundAnimation>,
    animation_colors: Vec<Color>,
}

const COLOR_PROPERTIES: [&str; 19] = [
    "window",
    "card",
    "border",
    "strong-border",
    "text",
    "dimmed",
    "accent",
    "on-accent",
    "accent-text",
    "raised",
    "hovered",
    "pressed",
    "field",
    "dialog",
    "shade",
    "title-bar",
    "destructive",
    "success",
    "warning",
];

pub fn parse_theme(file_name: &str, css: &str) -> Result<ImportedTheme, String> {
    let mut declared = Declared::default();
    let mut problems = Vec::new();
    let mut understood = 0;

    for (property, value) in declarations(css) {
        match declare(&mut declared, &property, &value) {
            Ok(()) => understood += 1,
            Err(problem) => problems.push(format!("--{property}: {problem}")),
        }
    }
    if understood == 0 {
        return Err(if problems.is_empty() {
            "The file doesn't set any theme properties, like --window or --accent.".to_owned()
        } else {
            format!(
                "None of its properties could be used: {}",
                problems.join("; ")
            )
        });
    }

    Ok(ImportedTheme {
        name: declared
            .name
            .clone()
            .unwrap_or_else(|| file_name.to_owned()),
        look: look_from(&declared),
        background: declared.background.clone(),
        animation: declared.animation.unwrap_or(BackgroundAnimation::None),
        animation_colors: declared.animation_colors.clone(),
        problems,
    })
}

fn declarations(css: &str) -> Vec<(String, String)> {
    without_comments(css)
        .replace(['{', '}'], ";")
        .split(';')
        .filter_map(|piece| {
            let (property, value) = piece.trim().split_once(':')?;
            let property = property.trim().strip_prefix("--")?;
            Some((property.to_ascii_lowercase(), value.trim().to_owned()))
        })
        .collect()
}

fn without_comments(css: &str) -> String {
    let mut kept = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(start) = rest.find("/*") {
        kept.push_str(&rest[..start]);
        rest = rest[start + 2..]
            .split_once("*/")
            .map_or("", |(_, after)| after);
    }
    kept.push_str(rest);
    kept
}

fn declare(declared: &mut Declared, property: &str, value: &str) -> Result<(), String> {
    if let Some(&name) = COLOR_PROPERTIES.iter().find(|&&name| name == property) {
        let color = parse_color(value).ok_or("isn't a colour")?;
        declared.colors.push((name, color));
        return Ok(());
    }
    match property {
        "name" => declared.name = Some(unquoted(value).to_owned()).filter(|name| !name.is_empty()),
        "radius" => declared.radius = Some(parse_length(value).ok_or("isn't a length")?),
        "font" => declared.font = Some(first_family(value).ok_or("isn't a font name")?),
        "mono-font" => declared.mono_font = Some(first_family(value).ok_or("isn't a font name")?),
        "background" => declared.background = Some(parse_background(value)?),
        "animation" => {
            declared.animation = Some(
                BackgroundAnimation::named(value.trim())
                    .filter(|&animation| animation != BackgroundAnimation::ThemeDefault)
                    .ok_or("isn't an animation Quadra knows")?,
            )
        }
        "animation-colors" => {
            declared.animation_colors = split_list(value)
                .iter()
                .take(MOST_ANIMATION_COLORS)
                .map(|color| parse_color(color).ok_or(format!("{color} isn't a colour")))
                .collect::<Result<_, _>>()?
        }
        _ => return Err("isn't a property Quadra knows".to_owned()),
    }
    Ok(())
}

fn look_from(declared: &Declared) -> Look {
    let declared_color = |name: &str| {
        declared
            .colors
            .iter()
            .rev()
            .find(|(declared_name, _)| *declared_name == name)
            .map(|&(_, color)| color)
    };
    let window = declared_color("window");
    let dark = window.is_none_or(|window| brightness(window) < 0.5);
    let base = Look::adwaita(dark);
    let window = window.unwrap_or(base.window);
    let text = declared_color("text").unwrap_or(base.text);
    let accent = declared_color("accent").unwrap_or(base.accent);
    let toward_text = |amount| mix(window, text, amount);

    Look {
        dark,
        window,
        card: declared_color("card").unwrap_or(base.card),
        border: declared_color("border").unwrap_or(base.border),
        strong_border: declared_color("strong-border").unwrap_or(base.strong_border),
        text,
        dimmed: declared_color("dimmed").unwrap_or_else(|| toward_text(0.6)),
        accent,
        on_accent: declared_color("on-accent").unwrap_or(if brightness(accent) > 0.6 {
            Color::BLACK
        } else {
            Color::WHITE
        }),
        accent_text: declared_color("accent-text").unwrap_or(accent),
        raised: declared_color("raised").unwrap_or(base.raised),
        hovered: declared_color("hovered").unwrap_or(base.hovered),
        pressed: declared_color("pressed").unwrap_or(base.pressed),
        field: declared_color("field").unwrap_or(base.field),
        dialog: declared_color("dialog").unwrap_or_else(|| toward_text(0.07)),
        shade: declared_color("shade").unwrap_or(base.shade),
        bar: declared_color("title-bar").unwrap_or_else(|| toward_text(0.04)),
        destructive: declared_color("destructive").unwrap_or(base.destructive),
        success: declared_color("success").unwrap_or(base.success),
        warning: declared_color("warning").unwrap_or(base.warning),
        radius: declared.radius.unwrap_or(base.radius),
        font: declared.font.as_deref().map_or(base.font, font_named),
        mono: declared.mono_font.as_deref().map_or(base.mono, font_named),
    }
}

fn font_named(family: &str) -> Font {
    Font::with_name(crate::utils::lasting_name(family))
}

pub fn brightness(color: Color) -> f32 {
    0.2126 * color.r + 0.7152 * color.g + 0.0722 * color.b
}

fn mix(from: Color, to: Color, amount: f32) -> Color {
    Color::from_rgba(
        from.r + (to.r - from.r) * amount,
        from.g + (to.g - from.g) * amount,
        from.b + (to.b - from.b) * amount,
        from.a + (to.a - from.a) * amount,
    )
}

fn unquoted(value: &str) -> &str {
    value
        .trim()
        .trim_matches(|character| character == '"' || character == '\'')
}

fn first_family(value: &str) -> Option<String> {
    let family = unquoted(value.split(',').next()?.trim());
    (!family.is_empty()).then(|| family.to_owned())
}

fn parse_length(value: &str) -> Option<f32> {
    let value = value.trim();
    let number = value.strip_suffix("px").unwrap_or(value).trim();
    number
        .parse::<f32>()
        .ok()
        .filter(|length| (0.0..=64.0).contains(length))
}

fn split_list(value: &str) -> Vec<String> {
    let mut items = Vec::new();
    let mut depth = 0;
    let mut current = String::new();
    for character in value.chars() {
        match character {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                items.push(current.trim().to_owned());
                current.clear();
                continue;
            }
            _ => {}
        }
        current.push(character);
    }
    if !current.trim().is_empty() {
        items.push(current.trim().to_owned());
    }
    items
}

pub fn parse_color(value: &str) -> Option<Color> {
    let value = value.trim().to_ascii_lowercase();
    if let Some(hex) = value.strip_prefix('#') {
        return parse_hex(hex);
    }
    if let Some(arguments) = value
        .strip_prefix("rgba(")
        .or_else(|| value.strip_prefix("rgb("))
        .and_then(|rest| rest.strip_suffix(')'))
    {
        return parse_rgb_arguments(arguments);
    }
    match value.as_str() {
        "white" => Some(Color::WHITE),
        "black" => Some(Color::BLACK),
        "transparent" => Some(Color::TRANSPARENT),
        _ => None,
    }
}

fn parse_hex(hex: &str) -> Option<Color> {
    if !hex.chars().all(|character| character.is_ascii_hexdigit()) {
        return None;
    }
    let digit = |index: usize| u8::from_str_radix(&hex[index..=index], 16).ok();
    let pair = |index: usize| u8::from_str_radix(&hex[index..index + 2], 16).ok();
    let channels: Vec<u8> = match hex.len() {
        3 | 4 => (0..hex.len())
            .map(|index| digit(index).map(|value| value * 17))
            .collect::<Option<_>>()?,
        6 | 8 => (0..hex.len()).step_by(2).map(pair).collect::<Option<_>>()?,
        _ => return None,
    };
    let alpha = channels.get(3).copied().unwrap_or(255);
    Some(Color::from_rgba8(
        channels[0],
        channels[1],
        channels[2],
        f32::from(alpha) / 255.0,
    ))
}

fn parse_rgb_arguments(arguments: &str) -> Option<Color> {
    let parts: Vec<&str> = arguments
        .split([',', '/', ' '])
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();
    if !(3..=4).contains(&parts.len()) {
        return None;
    }
    let channel = |part: &str| match part.strip_suffix('%') {
        Some(percent) => percent.parse::<f32>().ok().map(|percent| percent / 100.0),
        None => part.parse::<f32>().ok().map(|value| value / 255.0),
    };
    let alpha = match parts.get(3) {
        Some(part) => match part.strip_suffix('%') {
            Some(percent) => percent.parse::<f32>().ok()? / 100.0,
            None => part.parse::<f32>().ok()?,
        },
        None => 1.0,
    };
    Some(Color::from_rgba(
        channel(parts[0])?.clamp(0.0, 1.0),
        channel(parts[1])?.clamp(0.0, 1.0),
        channel(parts[2])?.clamp(0.0, 1.0),
        alpha.clamp(0.0, 1.0),
    ))
}

pub fn hex(color: Color) -> String {
    let [red, green, blue, _] = color.into_rgba8();
    format!("#{red:02x}{green:02x}{blue:02x}")
}

fn parse_background(value: &str) -> Result<Glow, String> {
    if let Some(color) = parse_color(value) {
        return Ok(Glow::new(Degrees(180.0), vec![(0.0, color), (1.0, color)]));
    }
    let arguments = value
        .trim()
        .strip_prefix("linear-gradient(")
        .and_then(|rest| rest.strip_suffix(')'))
        .ok_or("should be a colour or a linear-gradient(…)")?;
    let mut items = split_list(arguments);
    let angle = match items.first().and_then(|first| parse_direction(first)) {
        Some(angle) => {
            items.remove(0);
            angle
        }
        None => 180.0,
    };
    if !(2..=MOST_GRADIENT_STOPS).contains(&items.len()) {
        return Err(format!("needs 2 to {MOST_GRADIENT_STOPS} colours"));
    }
    let last = (items.len() - 1) as f32;
    let stops = items
        .iter()
        .enumerate()
        .map(|(position, item)| {
            let (color, offset) = match item.rsplit_once(' ') {
                Some((color, percent)) if percent.ends_with('%') => {
                    let offset = percent.trim_end_matches('%').parse::<f32>().ok();
                    (color, offset.map(|percent| percent / 100.0))
                }
                _ => (item.as_str(), None),
            };
            let color = parse_color(color).ok_or(format!("{color} isn't a colour"))?;
            Ok((
                offset.unwrap_or(position as f32 / last).clamp(0.0, 1.0),
                color,
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(Glow::new(Degrees(angle), stops))
}

fn parse_direction(value: &str) -> Option<f32> {
    let value = value.trim();
    if let Some(degrees) = value.strip_suffix("deg") {
        return degrees.trim().parse().ok();
    }
    match value {
        "to top" => Some(0.0),
        "to right" => Some(90.0),
        "to bottom" => Some(180.0),
        "to left" => Some(270.0),
        "to bottom right" | "to right bottom" => Some(135.0),
        "to bottom left" | "to left bottom" => Some(225.0),
        "to top left" | "to left top" => Some(315.0),
        "to top right" | "to right top" => Some(45.0),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{BackgroundAnimation, hex, parse_color, parse_theme};
    use iced::Color;

    const MIDNIGHT: &str = r#"
        /* A dark blue theme */
        :root {
            --name: "Midnight";
            --window: #101418;
            --accent: rgb(88, 166, 255);
            --radius: 10px;
            --font: "Inter", sans-serif;
            --background: linear-gradient(135deg, #1b2735, #090a0f 80%);
            --animation: waves;
            --animation-colors: #58a6ff, rgba(0, 210, 255, 0.8);
        }
    "#;

    #[test]
    fn reads_a_whole_theme() {
        let theme = parse_theme("midnight", MIDNIGHT).unwrap();
        assert_eq!(theme.name, "Midnight");
        assert!(theme.look.dark);
        assert_eq!(hex(theme.look.window), "#101418");
        assert_eq!(hex(theme.look.accent), "#58a6ff");
        assert_eq!(theme.look.radius, 10.0);
        assert_eq!(theme.animation, BackgroundAnimation::Waves);
        assert_eq!(theme.animation_colors.len(), 2);
        assert!(theme.background.is_some());
        assert!(theme.problems.is_empty(), "{:?}", theme.problems);
    }

    #[test]
    fn light_windows_start_from_the_light_look() {
        let theme = parse_theme("sand", ":root { --window: #f4ecd8; }").unwrap();
        assert!(!theme.look.dark);
        assert_eq!(theme.name, "sand");
        assert_eq!(theme.animation, BackgroundAnimation::None);
    }

    #[test]
    fn bad_values_are_skipped_and_reported() {
        let theme = parse_theme(
            "odd",
            ":root { --accent: banana; --window: #000; --sparkle: 1; }",
        )
        .unwrap();
        assert_eq!(theme.problems.len(), 2);
        assert_eq!(hex(theme.look.window), "#000000");
    }

    #[test]
    fn a_file_without_theme_properties_is_refused() {
        assert!(parse_theme("empty", "body { color: red; }").is_err());
        assert!(parse_theme("broken", ":root { --accent: nope; }").is_err());
    }

    #[test]
    fn reads_colour_notations() {
        assert_eq!(parse_color("#fff"), Some(Color::WHITE));
        assert_eq!(
            parse_color("#00000080").map(|color| (color.a * 255.0).round()),
            Some(128.0)
        );
        assert_eq!(
            parse_color("rgb(255 0 0)"),
            Some(Color::from_rgb(1.0, 0.0, 0.0))
        );
        assert_eq!(
            parse_color("rgba(0, 0, 0, 50%)").map(|color| color.a),
            Some(0.5)
        );
        assert_eq!(parse_color("#12345"), None);
        assert_eq!(parse_color("#ggg"), None);
    }
}
