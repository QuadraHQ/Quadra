use super::SettingsMessage;
use super::bound_rows::choice_row;
use crate::components::icon::Symbol;
use crate::components::imported_theme::{hex, parse_color};
use crate::components::preferences::{icon_button, preference_row};
use crate::components::style::{self, Look};
use crate::components::theme_backdrop::{chosen_scene, theme_colors_for};
use crate::quadra::{Message, act};
use crate::state::{AppState, Choice, Theme};
use iced::widget::{container, pick_list, row, space, text_input};
use iced::{Alignment, Background, Border, Element, Length};
use std::fmt;

const COLOR_INPUT_WIDTH: f32 = 96.0;
const SWATCH_SIZE: f32 = 18.0;

#[derive(Debug, Clone, PartialEq, Eq)]
enum ThemeOption {
    BuiltIn(Theme),
    Imported(String),
}

impl fmt::Display for ThemeOption {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BuiltIn(theme) => formatter.write_str(theme.label()),
            Self::Imported(name) => write!(formatter, "{name} (imported)"),
        }
    }
}

pub fn appearance_rows<'a>(state: &AppState, look: Look) -> Vec<Element<'a, Message>> {
    let mut rows = vec![
        theme_row(state, look),
        choice_row(
            look,
            "Background animation",
            state.background_animation(),
            |state, animation| state.set_background_animation(animation),
        )
        .subtitle("Animations pause while Quadra isn't focused or in use")
        .into(),
    ];
    rows.extend(color_rows(state, look));
    rows
}

fn theme_row<'a>(state: &AppState, look: Look) -> Element<'a, Message> {
    let imported = state.imported_themes();
    let chosen_import = state.imported_theme();
    let chosen = match imported.named(&chosen_import) {
        Some(theme) => ThemeOption::Imported(theme.name.clone()),
        None => ThemeOption::BuiltIn(state.theme()),
    };
    let options: Vec<ThemeOption> = Theme::ALL
        .iter()
        .copied()
        .map(ThemeOption::BuiltIn)
        .chain(
            imported
                .0
                .iter()
                .map(|theme| ThemeOption::Imported(theme.name.clone())),
        )
        .collect();
    let picker = pick_list(options, Some(chosen.clone()), |option| {
        act(move |state| match &option {
            ThemeOption::BuiltIn(theme) => {
                state.set_imported_theme("");
                state.set_theme(*theme);
            }
            ThemeOption::Imported(name) => state.set_imported_theme(name.as_str()),
        })
    })
    .font(look.font)
    .text_size(14)
    .style(style::choice(look))
    .menu_style(style::menu(look));

    let mut theme_row = preference_row(look, "App theme")
        .subtitle("Pick a theme, or import your own written in CSS")
        .suffix(picker)
        .suffix(icon_button(
            look,
            Symbol::Add,
            "Import a theme from a CSS file",
            Some(Message::Settings(SettingsMessage::ImportTheme)),
        ));
    if let ThemeOption::Imported(name) = chosen {
        theme_row = theme_row.suffix(icon_button(
            look,
            Symbol::Trash,
            "Remove this imported theme",
            Some(Message::Settings(SettingsMessage::RemoveTheme(name))),
        ));
    }
    theme_row.into()
}

fn color_rows<'a>(state: &AppState, look: Look) -> Vec<Element<'a, Message>> {
    let Some(scene) = chosen_scene(state) else {
        return Vec::new();
    };
    let typed = state.animation_colors();
    let defaults = theme_colors_for(state, scene);
    let names = scene.light_names();
    let anything_typed = typed.iter().any(|color| !color.is_empty());

    names
        .iter()
        .enumerate()
        .map(|(index, &name)| {
            let text = typed.get(index).cloned().unwrap_or_default();
            let parsed = parse_color(&text);
            let shown = parsed.unwrap_or(defaults[index]);
            let input = text_input(&hex(defaults[index]), &text)
                .font(look.mono)
                .size(13)
                .width(COLOR_INPUT_WIDTH)
                .padding([6, 10])
                .style(style::input(look, text.is_empty() || parsed.is_some()))
                .on_input(move |text| {
                    Message::Settings(SettingsMessage::AnimationColorEdited(index, text))
                });
            let swatch =
                container(space().width(SWATCH_SIZE).height(SWATCH_SIZE)).style(move |_| {
                    container::Style {
                        background: Some(Background::Color(shown)),
                        border: Border {
                            color: look.strong_border,
                            width: 1.0,
                            radius: (SWATCH_SIZE / 2.0).into(),
                        },
                        ..container::Style::default()
                    }
                });
            let mut color_row = preference_row(look, format!("Animation colour: {name}"))
                .suffix(row![swatch, input].spacing(8).align_y(Alignment::Center));
            if index == 0 {
                color_row =
                    color_row.subtitle("Leave empty for the theme's colour. Use #rrggbb or rgb(…)");
                if anything_typed {
                    color_row = color_row.suffix(icon_button(
                        look,
                        Symbol::Clear,
                        "Use the theme's colours",
                        Some(act(|state| state.set_animation_colors(Vec::new()))),
                    ));
                }
            }
            container(color_row).width(Length::Fill).into()
        })
        .collect()
}
