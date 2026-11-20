use crate::state::{AppState, Theme};
use iced::widget::{
    button, container, pick_list, progress_bar, rule, scrollable, text, text_editor, text_input,
    toggler,
};
use iced::{Background, Border, Color, Font, Shadow, Vector, color, overlay, theme};

pub const ARCHIVO: Font = Font::with_name("Archivo");
pub const JETBRAINS_MONO: Font = Font::with_name("JetBrains Mono");

pub const BODY_SIZE: f32 = 14.0;
pub const CAPTION_SIZE: f32 = 12.0;
pub const HEADING_SIZE: f32 = 12.0;
pub const TITLE_SIZE: f32 = 26.0;
pub const DIALOG_TITLE_SIZE: f32 = 17.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Look {
    pub dark: bool,
    pub window: Color,
    pub card: Color,
    pub border: Color,
    pub strong_border: Color,
    pub text: Color,
    pub dimmed: Color,
    pub accent: Color,
    pub on_accent: Color,
    pub accent_text: Color,
    pub raised: Color,
    pub hovered: Color,
    pub pressed: Color,
    pub field: Color,
    pub dialog: Color,
    pub shade: Color,
    pub bar: Color,
    pub destructive: Color,
    pub success: Color,
    pub warning: Color,
    pub radius: f32,
    pub font: Font,
    pub mono: Font,
}

impl Look {
    pub fn of(state: &AppState) -> Self {
        Self::for_theme(state.theme(), state.system_prefers_dark())
    }

    pub fn for_theme(theme: Theme, system_prefers_dark: bool) -> Self {
        match theme {
            Theme::Adwaita if system_prefers_dark => ADWAITA_DARK,
            Theme::Adwaita => ADWAITA_LIGHT,
            Theme::Grid => GRID,
            Theme::Monochrome => MONOCHROME,
            Theme::Aurora => AURORA,
            Theme::Ember => EMBER,
            Theme::Paper => PAPER,
            Theme::Phosphor => PHOSPHOR,
        }
    }

    pub fn iced_theme(&self) -> iced::Theme {
        let name = if self.dark {
            "Quadra dark"
        } else {
            "Quadra light"
        };
        iced::Theme::custom(
            name,
            theme::Palette {
                background: self.window,
                text: self.text,
                primary: self.accent,
                success: self.success,
                warning: self.warning,
                danger: self.destructive,
            },
        )
    }

    pub fn app_style(&self) -> theme::Style {
        theme::Style {
            background_color: self.window,
            text_color: self.text,
        }
    }

    fn border(&self, color: Color) -> Border {
        Border {
            color,
            width: 1.0,
            radius: self.radius.into(),
        }
    }
}

const ADWAITA_LIGHT: Look = Look {
    dark: false,
    window: color!(0xfafafb),
    card: color!(0xffffff),
    border: Color::from_rgba(0.0, 0.0, 0.024, 0.09),
    strong_border: Color::from_rgba(0.0, 0.0, 0.024, 0.18),
    text: color!(0x323233),
    dimmed: color!(0x707072),
    accent: color!(0x3584e4),
    on_accent: Color::WHITE,
    accent_text: color!(0x1c71d8),
    raised: Color::from_rgba(0.0, 0.0, 0.024, 0.08),
    hovered: Color::from_rgba(0.0, 0.0, 0.024, 0.12),
    pressed: Color::from_rgba(0.0, 0.0, 0.024, 0.2),
    field: Color::from_rgba(0.0, 0.0, 0.024, 0.06),
    dialog: color!(0xffffff),
    shade: Color::from_rgba(0.0, 0.0, 0.0, 0.3),
    bar: color!(0xebebed),
    destructive: color!(0xe01b24),
    success: color!(0x2ec27e),
    warning: color!(0xe5a50a),
    radius: 8.0,
    font: Font::DEFAULT,
    mono: Font::MONOSPACE,
};

const ADWAITA_DARK: Look = Look {
    dark: true,
    window: color!(0x222226),
    card: Color::from_rgba(1.0, 1.0, 1.0, 0.08),
    border: Color::from_rgba(0.0, 0.0, 0.024, 0.36),
    strong_border: Color::from_rgba(1.0, 1.0, 1.0, 0.15),
    text: Color::WHITE,
    dimmed: color!(0x9b9b9e),
    accent: color!(0x3584e4),
    on_accent: Color::WHITE,
    accent_text: color!(0x78aeed),
    raised: Color::from_rgba(1.0, 1.0, 1.0, 0.1),
    hovered: Color::from_rgba(1.0, 1.0, 1.0, 0.15),
    pressed: Color::from_rgba(1.0, 1.0, 1.0, 0.25),
    field: Color::from_rgba(1.0, 1.0, 1.0, 0.08),
    dialog: color!(0x36363a),
    shade: Color::from_rgba(0.0, 0.0, 0.0, 0.5),
    bar: color!(0x2e2e32),
    destructive: color!(0xc01c28),
    success: color!(0x26a269),
    warning: color!(0xcd9309),
    radius: 8.0,
    font: Font::DEFAULT,
    mono: Font::MONOSPACE,
};

const GRID: Look = Look {
    dark: true,
    window: color!(0x10141b),
    card: Color::from_rgba(24.0 / 255.0, 33.0 / 255.0, 44.0 / 255.0, 0.94),
    border: color!(0x263343),
    strong_border: color!(0x344458),
    text: color!(0xedf2f8),
    dimmed: color!(0xacb9ca),
    accent: color!(0x9cc3ff),
    on_accent: color!(0x0c1829),
    accent_text: color!(0xa6c8ff),
    raised: color!(0x202b38),
    hovered: color!(0x2a3747),
    pressed: Color::from_rgba(240.0 / 255.0, 240.0 / 255.0, 240.0 / 255.0, 0.12),
    field: Color::from_rgba(0.0, 0.0, 0.0, 0.45),
    dialog: color!(0x18212c),
    shade: Color::from_rgba(0.0, 0.0, 0.0, 0.7),
    bar: Color::from_rgba(16.0 / 255.0, 20.0 / 255.0, 27.0 / 255.0, 0.86),
    destructive: color!(0xedf2f8),
    success: color!(0xa6c8ff),
    warning: color!(0xb0c3de),
    radius: 0.0,
    font: ARCHIVO,
    mono: JETBRAINS_MONO,
};

const MONOCHROME: Look = Look {
    dark: true,
    window: color!(0x000000),
    card: Color::from_rgba(30.0 / 255.0, 30.0 / 255.0, 30.0 / 255.0, 0.4),
    border: color!(0x1e1e1e),
    strong_border: color!(0x2f2f2f),
    text: color!(0xe8e8e8),
    dimmed: color!(0x9a9a9a),
    accent: color!(0xf0f0f0),
    on_accent: color!(0x000000),
    accent_text: color!(0xf0f0f0),
    raised: color!(0x1c1c1c),
    hovered: color!(0x262626),
    pressed: Color::from_rgba(240.0 / 255.0, 240.0 / 255.0, 240.0 / 255.0, 0.12),
    field: Color::from_rgba(0.0, 0.0, 0.0, 0.45),
    dialog: color!(0x121212),
    shade: Color::from_rgba(0.0, 0.0, 0.0, 0.7),
    bar: Color::from_rgba(0.0, 0.0, 0.0, 0.86),
    destructive: color!(0xe8e8e8),
    success: color!(0xf0f0f0),
    warning: color!(0xbdbdbd),
    radius: 0.0,
    font: ARCHIVO,
    mono: JETBRAINS_MONO,
};

const AURORA: Look = Look {
    dark: true,
    window: color!(0x140d24),
    card: Color::from_rgba(1.0, 1.0, 1.0, 0.06),
    border: Color::from_rgba(196.0 / 255.0, 160.0 / 255.0, 1.0, 0.16),
    strong_border: Color::from_rgba(196.0 / 255.0, 160.0 / 255.0, 1.0, 0.3),
    text: color!(0xf4eefe),
    dimmed: color!(0xb5a8d2),
    accent: color!(0xff5fa2),
    on_accent: color!(0x24081c),
    accent_text: color!(0xff9ccb),
    raised: Color::from_rgba(1.0, 1.0, 1.0, 0.08),
    hovered: Color::from_rgba(1.0, 1.0, 1.0, 0.13),
    pressed: Color::from_rgba(1.0, 1.0, 1.0, 0.2),
    field: Color::from_rgba(0.0, 0.0, 0.0, 0.25),
    dialog: color!(0x1e1535),
    shade: Color::from_rgba(5.0 / 255.0, 2.0 / 255.0, 12.0 / 255.0, 0.6),
    bar: Color::from_rgba(20.0 / 255.0, 13.0 / 255.0, 36.0 / 255.0, 0.55),
    destructive: color!(0xff7a7a),
    success: color!(0x7ee8b8),
    warning: color!(0xffd36e),
    radius: 12.0,
    font: Font::DEFAULT,
    mono: JETBRAINS_MONO,
};

const EMBER: Look = Look {
    dark: true,
    window: color!(0x16110e),
    card: color!(0x211913),
    border: color!(0x2f251e),
    strong_border: color!(0x403228),
    text: color!(0xf5ebe3),
    dimmed: color!(0xb4a597),
    accent: color!(0xff8a3d),
    on_accent: color!(0x1f0f04),
    accent_text: color!(0xffae75),
    raised: color!(0x2b2119),
    hovered: color!(0x352920),
    pressed: color!(0x403128),
    field: Color::from_rgba(0.0, 0.0, 0.0, 0.3),
    dialog: color!(0x221a14),
    shade: Color::from_rgba(0.0, 0.0, 0.0, 0.6),
    bar: Color::from_rgba(22.0 / 255.0, 17.0 / 255.0, 14.0 / 255.0, 0.7),
    destructive: color!(0xff6d5a),
    success: color!(0x9ad48f),
    warning: color!(0xf2c14e),
    radius: 10.0,
    font: Font::DEFAULT,
    mono: JETBRAINS_MONO,
};

const PAPER: Look = Look {
    dark: false,
    window: color!(0xf5f0e6),
    card: color!(0xfffcf6),
    border: Color::from_rgba(60.0 / 255.0, 45.0 / 255.0, 20.0 / 255.0, 0.12),
    strong_border: Color::from_rgba(60.0 / 255.0, 45.0 / 255.0, 20.0 / 255.0, 0.24),
    text: color!(0x2b2620),
    dimmed: color!(0x6f6558),
    accent: color!(0x1f6f6b),
    on_accent: color!(0xffffff),
    accent_text: color!(0x1a5c58),
    raised: Color::from_rgba(60.0 / 255.0, 45.0 / 255.0, 20.0 / 255.0, 0.07),
    hovered: Color::from_rgba(60.0 / 255.0, 45.0 / 255.0, 20.0 / 255.0, 0.11),
    pressed: Color::from_rgba(60.0 / 255.0, 45.0 / 255.0, 20.0 / 255.0, 0.17),
    field: Color::from_rgba(60.0 / 255.0, 45.0 / 255.0, 20.0 / 255.0, 0.05),
    dialog: color!(0xfffcf6),
    shade: Color::from_rgba(40.0 / 255.0, 30.0 / 255.0, 15.0 / 255.0, 0.25),
    bar: color!(0xede6d8),
    destructive: color!(0xb3261e),
    success: color!(0x2e7d4f),
    warning: color!(0xb7791f),
    radius: 6.0,
    font: ARCHIVO,
    mono: JETBRAINS_MONO,
};

const PHOSPHOR: Look = Look {
    dark: true,
    window: color!(0x030805),
    card: Color::from_rgba(10.0 / 255.0, 30.0 / 255.0, 16.0 / 255.0, 0.78),
    border: color!(0x12361d),
    strong_border: color!(0x1d5a30),
    text: color!(0xa6ffbf),
    dimmed: color!(0x52a86b),
    accent: color!(0x39ff7a),
    on_accent: color!(0x001a08),
    accent_text: color!(0x6dff9c),
    raised: color!(0x0b2213),
    hovered: color!(0x103019),
    pressed: color!(0x174022),
    field: Color::from_rgba(0.0, 0.0, 0.0, 0.5),
    dialog: color!(0x061309),
    shade: Color::from_rgba(0.0, 0.0, 0.0, 0.75),
    bar: Color::from_rgba(3.0 / 255.0, 8.0 / 255.0, 5.0 / 255.0, 0.85),
    destructive: color!(0xff6b6b),
    success: color!(0x39ff7a),
    warning: color!(0xe6ff5c),
    radius: 0.0,
    font: JETBRAINS_MONO,
    mono: JETBRAINS_MONO,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Emphasis {
    Normal,
    Suggested,
    Destructive,
    Flat,
}

pub fn emphasized(
    look: Look,
    emphasis: Emphasis,
) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_, status| {
        let (background, text_color, border) = match emphasis {
            Emphasis::Normal => (look.raised, look.text, look.border(Color::TRANSPARENT)),
            Emphasis::Suggested => (look.accent, look.on_accent, look.border(look.accent)),
            Emphasis::Destructive if look.radius == 0.0 => (
                Color::TRANSPARENT,
                look.destructive,
                look.border(look.destructive),
            ),
            Emphasis::Destructive => (
                look.raised,
                look.destructive,
                look.border(Color::TRANSPARENT),
            ),
            Emphasis::Flat => (
                Color::TRANSPARENT,
                look.text,
                look.border(Color::TRANSPARENT),
            ),
        };
        let background = match (status, emphasis) {
            (button::Status::Hovered, Emphasis::Suggested) => lighten(background, 0.08),
            (button::Status::Pressed, Emphasis::Suggested) => darken(background, 0.08),
            (button::Status::Hovered, _) => look.hovered,
            (button::Status::Pressed, _) => look.pressed,
            _ => background,
        };
        let is_disabled = status == button::Status::Disabled;
        let fade = |color: Color| if is_disabled { faded(color) } else { color };
        button::Style {
            background: Some(Background::Color(fade(background))),
            text_color: fade(text_color),
            border,
            shadow: Shadow::default(),
            snap: true,
        }
    }
}

pub fn tab(look: Look, selected: bool) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_, status| {
        let background = match (selected, status) {
            (true, _) => look.pressed,
            (false, button::Status::Hovered) => look.hovered,
            (false, button::Status::Pressed) => look.pressed,
            (false, _) => Color::TRANSPARENT,
        };
        button::Style {
            background: Some(Background::Color(background)),
            text_color: if selected {
                look.accent_text
            } else {
                look.text
            },
            border: look.border(Color::TRANSPARENT),
            shadow: Shadow::default(),
            snap: true,
        }
    }
}

pub fn window_button(look: Look) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_, status| {
        let background = match status {
            button::Status::Hovered => look.hovered,
            button::Status::Pressed => look.pressed,
            _ => look.raised,
        };
        button::Style {
            background: Some(Background::Color(background)),
            text_color: look.text,
            border: Border {
                radius: if look.radius > 0.0 { 100.0 } else { 0.0 }.into(),
                ..Border::default()
            },
            shadow: Shadow::default(),
            snap: true,
        }
    }
}

pub fn outline(look: Look) -> impl Fn(&iced::Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(look.strong_border)),
        ..container::Style::default()
    }
}

pub fn card(look: Look) -> impl Fn(&iced::Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(look.card)),
        border: Border {
            color: if look.dark {
                look.border
            } else {
                Color::TRANSPARENT
            },
            width: 1.0,
            radius: (look.radius + 4.0 * f32::from(look.radius > 0.0)).into(),
        },
        shadow: if look.dark {
            Shadow::default()
        } else {
            Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.024, 0.08),
                offset: Vector::new(0.0, 1.0),
                blur_radius: 3.0,
            }
        },
        ..container::Style::default()
    }
}

pub fn code_block(look: Look) -> impl Fn(&iced::Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(look.field)),
        border: look.border(look.border),
        text_color: Some(look.text),
        ..container::Style::default()
    }
}

pub fn dialog(look: Look) -> impl Fn(&iced::Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(look.dialog)),
        border: Border {
            color: look.strong_border,
            width: f32::from(look.dark),
            radius: (look.radius * 1.5).into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.4),
            offset: Vector::new(0.0, 8.0),
            blur_radius: 32.0,
        },
        text_color: Some(look.text),
        ..container::Style::default()
    }
}

pub fn shade(look: Look) -> impl Fn(&iced::Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(look.shade)),
        ..container::Style::default()
    }
}

pub fn bar(look: Look) -> impl Fn(&iced::Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(look.bar)),
        ..container::Style::default()
    }
}

pub fn banner(look: Look) -> impl Fn(&iced::Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(look.raised)),
        text_color: Some(look.text),
        ..container::Style::default()
    }
}

pub fn tooltip(look: Look) -> impl Fn(&iced::Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(look.dialog)),
        border: look.border(look.strong_border),
        text_color: Some(look.text),
        ..container::Style::default()
    }
}

pub fn separator(look: Look) -> impl Fn(&iced::Theme) -> rule::Style {
    move |_| rule::Style {
        color: look.border,
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
        snap: true,
    }
}

pub fn dimmed(look: Look) -> impl Fn(&iced::Theme) -> text::Style {
    move |_| text::Style {
        color: Some(look.dimmed),
    }
}

pub fn colored(color: Color) -> impl Fn(&iced::Theme) -> text::Style {
    move |_| text::Style { color: Some(color) }
}

pub fn input(
    look: Look,
    is_valid: bool,
) -> impl Fn(&iced::Theme, text_input::Status) -> text_input::Style {
    move |_, status| {
        let border_color = match status {
            _ if !is_valid => look.destructive,
            text_input::Status::Focused { .. } => look.accent,
            text_input::Status::Hovered => look.strong_border,
            _ => look.border,
        };
        text_input::Style {
            background: Background::Color(look.field),
            border: Border {
                color: border_color,
                width: 1.0 + f32::from(!is_valid),
                radius: (look.radius * 0.75).into(),
            },
            icon: look.dimmed,
            placeholder: look.dimmed,
            value: look.text,
            selection: Color {
                a: 0.3,
                ..look.accent
            },
        }
    }
}

pub fn editor(look: Look) -> impl Fn(&iced::Theme, text_editor::Status) -> text_editor::Style {
    move |_, status| text_editor::Style {
        background: Background::Color(look.field),
        border: look.border(match status {
            text_editor::Status::Focused { .. } => look.accent,
            _ => look.border,
        }),
        placeholder: look.dimmed,
        value: look.text,
        selection: Color {
            a: 0.3,
            ..look.accent
        },
    }
}

pub fn log(look: Look) -> impl Fn(&iced::Theme, text_editor::Status) -> text_editor::Style {
    move |_, _| text_editor::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border::default(),
        placeholder: look.dimmed,
        value: look.text,
        selection: Color {
            a: 0.3,
            ..look.accent
        },
    }
}

pub fn switch(look: Look) -> impl Fn(&iced::Theme, toggler::Status) -> toggler::Style {
    move |_, status| {
        let (is_toggled, is_disabled, is_hovered) = match status {
            toggler::Status::Active { is_toggled } => (is_toggled, false, false),
            toggler::Status::Hovered { is_toggled } => (is_toggled, false, true),
            toggler::Status::Disabled { is_toggled } => (is_toggled, true, false),
        };
        let background = if is_toggled {
            look.accent
        } else {
            look.pressed
        };
        let background = if is_hovered {
            lighten(background, 0.05)
        } else {
            background
        };
        let foreground = match (is_toggled, look.radius > 0.0) {
            (_, true) => Color::WHITE,
            (true, false) => look.on_accent,
            (false, false) => look.dimmed,
        };
        let fade = |color: Color| if is_disabled { faded(color) } else { color };
        toggler::Style {
            background: Background::Color(fade(background)),
            background_border_width: 0.0,
            background_border_color: Color::TRANSPARENT,
            foreground: Background::Color(fade(foreground)),
            foreground_border_width: 0.0,
            foreground_border_color: Color::TRANSPARENT,
            text_color: Some(look.text),
            border_radius: Some(if look.radius > 0.0 { 100.0 } else { 0.0 }.into()),
            padding_ratio: 0.12,
        }
    }
}

pub fn choice(look: Look) -> impl Fn(&iced::Theme, pick_list::Status) -> pick_list::Style {
    move |_, status| pick_list::Style {
        text_color: look.text,
        placeholder_color: look.dimmed,
        handle_color: look.dimmed,
        background: Background::Color(match status {
            pick_list::Status::Hovered | pick_list::Status::Opened { .. } => look.hovered,
            pick_list::Status::Active => look.raised,
        }),
        border: look.border(Color::TRANSPARENT),
    }
}

pub fn menu(look: Look) -> impl Fn(&iced::Theme) -> overlay::menu::Style {
    move |_| overlay::menu::Style {
        background: Background::Color(look.dialog),
        border: look.border(look.strong_border),
        text_color: look.text,
        selected_text_color: look.on_accent,
        selected_background: Background::Color(look.accent),
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.3),
            offset: Vector::new(0.0, 4.0),
            blur_radius: 12.0,
        },
    }
}

pub fn progress(look: Look) -> impl Fn(&iced::Theme) -> progress_bar::Style {
    move |_| progress_bar::Style {
        background: Background::Color(look.pressed),
        bar: Background::Color(look.accent),
        border: Border {
            radius: (look.radius * 0.5).into(),
            ..Border::default()
        },
    }
}

pub fn scrollbar(look: Look) -> impl Fn(&iced::Theme, scrollable::Status) -> scrollable::Style {
    move |theme, status| {
        let mut style = scrollable::default(theme, status);
        let rail = scrollable::Rail {
            background: None,
            border: Border::default(),
            scroller: scrollable::Scroller {
                background: Background::Color(match status {
                    scrollable::Status::Hovered { .. } | scrollable::Status::Dragged { .. } => {
                        look.strong_border
                    }
                    scrollable::Status::Active { .. } => look.border,
                }),
                border: Border {
                    radius: look.radius.into(),
                    ..Border::default()
                },
            },
        };
        style.vertical_rail = rail;
        style.horizontal_rail = rail;
        style.container = container::Style::default();
        style
    }
}

fn faded(color: Color) -> Color {
    Color {
        a: color.a * 0.45,
        ..color
    }
}

fn lighten(color: Color, amount: f32) -> Color {
    Color {
        r: (color.r + amount).min(1.0),
        g: (color.g + amount).min(1.0),
        b: (color.b + amount).min(1.0),
        ..color
    }
}

fn darken(color: Color, amount: f32) -> Color {
    Color {
        r: (color.r - amount).max(0.0),
        g: (color.g - amount).max(0.0),
        b: (color.b - amount).max(0.0),
        ..color
    }
}
