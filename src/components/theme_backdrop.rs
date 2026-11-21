use super::gradient_backdrop::{self, Glow};
use super::imported_theme::parse_color;
use super::scene_backdrop::{Palette, Scene};
use super::style::Look;
use crate::state::{Animations, AppState, BackgroundAnimation, Theme};
use iced::Color;

#[derive(Debug, Clone, PartialEq)]
pub enum Still {
    Nothing,
    Gradient(Glow),
    Grid,
    LineField,
    Scanlines,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Backdrop {
    pub still: Still,
    pub scene: Option<(Scene, Palette)>,
    pub motion: Animations,
}

struct ThemeOwn {
    still: Still,
    scene: Option<Scene>,
    colors: Vec<Color>,
}

fn theme_own(state: &AppState) -> ThemeOwn {
    let imported_name = state.imported_theme();
    let has_import = state.imported_themes().named(&imported_name).is_some();
    let choice = if has_import {
        ThemeChoice::Imported(&imported_name)
    } else {
        ThemeChoice::BuiltIn(state.theme())
    };
    theme_own_for(state, choice)
}

fn theme_own_for(state: &AppState, choice: ThemeChoice<'_>) -> ThemeOwn {
    let imported = state.imported_themes();
    let built_in = match choice {
        ThemeChoice::BuiltIn(theme) => theme,
        ThemeChoice::Imported(name) => match imported.named(name) {
            Some(_) => Theme::Adwaita,
            None => state.theme(),
        },
    };
    if let ThemeChoice::Imported(name) = choice
        && let Some(theme) = imported.named(name)
    {
        return ThemeOwn {
            still: theme
                .background
                .clone()
                .map_or(Still::Nothing, Still::Gradient),
            scene: Scene::chosen(theme.animation),
            colors: theme.animation_colors.clone(),
        };
    }
    let (still, scene) = match built_in {
        Theme::Adwaita => (Still::Nothing, None),
        Theme::Grid => (Still::Grid, None),
        Theme::Monochrome => (Still::LineField, None),
        Theme::Aurora => (
            Still::Gradient(gradient_backdrop::AURORA),
            Some(Scene::Aurora),
        ),
        Theme::Ember => (
            Still::Gradient(gradient_backdrop::EMBER),
            Some(Scene::Embers),
        ),
        Theme::Paper => (Still::Nothing, Some(Scene::Leaves)),
        Theme::Phosphor => (Still::Scanlines, Some(Scene::Scanlines)),
    };
    ThemeOwn {
        still,
        scene,
        colors: Vec::new(),
    }
}

pub fn chosen_scene(state: &AppState) -> Option<Scene> {
    match state.background_animation() {
        BackgroundAnimation::ThemeDefault => theme_own(state).scene,
        animation => Scene::chosen(animation),
    }
}

pub fn chosen_backdrop(state: &AppState, look: &Look) -> Backdrop {
    backdrop_of(state, theme_own(state), look)
}

pub fn backdrop_for_theme(state: &AppState, theme: ThemeChoice<'_>, look: &Look) -> Backdrop {
    backdrop_of(state, theme_own_for(state, theme), look)
}

#[derive(Debug, Clone, Copy)]
pub enum ThemeChoice<'a> {
    BuiltIn(Theme),
    Imported(&'a str),
}

fn backdrop_of(state: &AppState, own: ThemeOwn, look: &Look) -> Backdrop {
    let motion = state.motion();
    let scene = match state.background_animation() {
        _ if motion == Animations::Off => None,
        BackgroundAnimation::ThemeDefault => own.scene,
        animation => Scene::chosen(animation),
    };
    let Some(scene) = scene else {
        let still = match (state.background_animation(), own.still) {
            (BackgroundAnimation::None, Still::LineField) => Still::Nothing,
            (_, still) => still,
        };
        return Backdrop {
            still,
            scene: None,
            motion,
        };
    };

    let picked = picked_colors(state);
    let fallback = theme_lights(&own, scene);
    let colors: Vec<Option<Color>> = fallback
        .iter()
        .enumerate()
        .map(|(index, &theme_color)| {
            Some(picked.get(index).copied().flatten().unwrap_or(theme_color))
        })
        .collect();
    let still = match own.still {
        Still::LineField => Still::Nothing,
        still => still,
    };
    Backdrop {
        still,
        scene: Some((scene, scene.palette(look.window, &colors))),
        motion,
    }
}

fn theme_lights(own: &ThemeOwn, scene: Scene) -> Vec<Color> {
    let defaults = scene.default_lights();
    (0..defaults.len())
        .map(|index| {
            own.colors
                .get(index)
                .filter(|_| own.scene == Some(scene))
                .copied()
                .unwrap_or(defaults[index])
        })
        .collect()
}

pub fn theme_colors_for(state: &AppState, scene: Scene) -> Vec<Color> {
    theme_lights(&theme_own(state), scene)
}

pub fn picked_colors(state: &AppState) -> Vec<Option<Color>> {
    state
        .animation_colors()
        .iter()
        .map(|text| parse_color(text))
        .collect()
}
