use super::gradient_backdrop::{self, Glow};
use super::imported_theme::parse_color;
use super::scene_backdrop::{Palette, Scene};
use super::style::Look;
use crate::state::{AppState, BackgroundAnimation, Theme};
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
}

struct ThemeOwn {
    still: Still,
    scene: Option<Scene>,
    colors: Vec<Color>,
}

fn theme_own(state: &AppState) -> ThemeOwn {
    let imported = state.imported_themes();
    if let Some(theme) = imported.named(&state.imported_theme()) {
        return ThemeOwn {
            still: theme
                .background
                .clone()
                .map_or(Still::Nothing, Still::Gradient),
            scene: Scene::chosen(theme.animation),
            colors: theme.animation_colors.clone(),
        };
    }
    let (still, scene) = match state.theme() {
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
    let own = theme_own(state);
    let scene = match state.background_animation() {
        BackgroundAnimation::ThemeDefault => own.scene,
        animation => Scene::chosen(animation),
    };
    let Some(scene) = scene else {
        let still = match (state.background_animation(), own.still) {
            (BackgroundAnimation::None, Still::LineField) => Still::Nothing,
            (_, still) => still,
        };
        return Backdrop { still, scene: None };
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
