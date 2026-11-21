use super::SettingsMessage;
use super::bound_rows::{SpinRange, choice_row, spin_row, switch_row};
use crate::components::file_row::{ChosenPath, Pick, file_row};
use crate::components::gpu_or::gpu_or;
use crate::components::gradient_backdrop::gradient_backdrop;
use crate::components::icon::{self, Symbol};
use crate::components::imported_theme::{hex, parse_color};
use crate::components::picker::{
    Card, Opener, adding_card, card_button, choosing, picker_card, picker_grid, picker_row,
};
use crate::components::preferences::{icon_button, preference_group, preference_row, searchable};
use crate::components::scene_backdrop::Scene;
use crate::components::style::{self, ARCHIVO, JETBRAINS_MONO, Look};
use crate::components::theme_backdrop::{
    Still, ThemeChoice, backdrop_for_theme, chosen_scene, picked_colors, theme_colors_for,
};
use crate::quadra::{Message, act};
use crate::state::{
    AppState, BACKGROUND_DIMMING_STEP, BACKGROUND_DIMMINGS, BackgroundAnimation,
    CURSOR_EFFECT_SIZE_STEP, CURSOR_EFFECT_SIZES, Choice, CursorEffect, Picker, TEXT_SIZE_STEP,
    TEXT_SIZES, Theme,
};
use crate::utils::{PRESET_FONTS, lasting_name};
use iced::font::Weight;
use iced::widget::{button, column, container, row, space, stack, text, text_input};
use iced::{Alignment, Background, Border, Color, Element, Font, Length};

const COLOR_INPUT_WIDTH: f32 = 96.0;
const SWATCH_SIZE: f32 = 18.0;
const CARDS_PER_ROW: usize = 3;
const CARD_SPACING: f32 = 10.0;
const PREVIEW_HEIGHT: f32 = 74.0;
const LABEL_HEIGHT: f32 = 30.0;
const ANIMATIONS_PER_ROW: usize = 4;
const ANIMATION_PREVIEW_HEIGHT: f32 = 58.0;
const FONT_PREVIEW_HEIGHT: f32 = 62.0;
const THEMES_OWN_FONT: &str = "Theme's own";
const PICTURE_FILES: &[Pick] = &[Pick::File(&["png", "jpg", "jpeg", "webp"])];

#[derive(Debug, Clone, PartialEq, Eq)]
enum ThemeOption {
    BuiltIn(Theme),
    Imported(String),
}

impl ThemeOption {
    fn name(&self) -> &str {
        match self {
            Self::BuiltIn(theme) => theme.label(),
            Self::Imported(name) => name,
        }
    }

    fn choice(&self) -> ThemeChoice<'_> {
        match self {
            Self::BuiltIn(theme) => ThemeChoice::BuiltIn(*theme),
            Self::Imported(name) => ThemeChoice::Imported(name),
        }
    }

    fn look(&self, state: &AppState) -> Look {
        match self {
            Self::BuiltIn(theme) => Look::for_theme(*theme, state.system_prefers_dark()),
            Self::Imported(name) => state
                .imported_themes()
                .named(name)
                .map_or_else(|| Look::of(state), |theme| theme.look),
        }
    }

    fn apply(&self) -> Message {
        let option = self.clone();
        act(move |state| match &option {
            ThemeOption::BuiltIn(theme) => {
                state.set_imported_theme("");
                state.set_theme(*theme);
            }
            ThemeOption::Imported(name) => state.set_imported_theme(name.as_str()),
        })
    }
}

pub fn appearance_group<'a>(state: &AppState, look: Look) -> Element<'a, Message> {
    let mut group = preference_group(look)
        .title("Appearance")
        .push(searchable(
            gallery(state, look),
            "App theme gallery import css",
        ))
        .push(picker_row(
            state,
            look,
            Opener {
                picker: Picker::Animation,
                title: "Background animation",
                subtitle: if has_picture(state) {
                    "Plays over your background picture. Hover one to play it"
                } else {
                    "Pick any animation for any theme. Hover one to play it"
                },
                shown: text(state.background_animation().label())
                    .font(look.font)
                    .size(look.sized(14.0))
                    .into(),
            },
        ))
        .push_maybe((state.open_picker() == Picker::Animation).then(|| {
            searchable(
                animation_picker(state, look),
                "Background animation picker preview",
            )
        }))
        .push(picker_row(
            state,
            look,
            Opener {
                picker: Picker::AppFont,
                title: "Font",
                subtitle: "The font Quadra's own text uses",
                shown: text(app_font_label(state))
                    .font(look.font)
                    .size(look.sized(14.0))
                    .into(),
            },
        ))
        .push_maybe(
            (state.open_picker() == Picker::AppFont)
                .then(|| searchable(app_font_picker(state, look), "Font typeface import font")),
        )
        .push(
            spin_row(
                look,
                "Text size (%)",
                state.text_size(),
                SpinRange {
                    values: TEXT_SIZES,
                    step: TEXT_SIZE_STEP,
                },
                |state, size| state.set_text_size(size),
                true,
            )
            .subtitle("Makes Quadra's text larger or smaller"),
        )
        .push(accent_row(state, look))
        .push(file_row(
            state,
            look,
            ChosenPath::BackgroundPicture,
            "Background picture",
            PICTURE_FILES,
        ))
        .push_maybe(has_picture(state).then(|| {
            spin_row(
                look,
                "Picture dimming (%)",
                state.background_dimming(),
                SpinRange {
                    values: BACKGROUND_DIMMINGS,
                    step: BACKGROUND_DIMMING_STEP,
                },
                |state, dimming| state.set_background_dimming(dimming),
                true,
            )
            .subtitle("Shades the picture in your theme's window colour so text stays readable")
        }))
        .push(
            choice_row(
                look,
                "Animations",
                state.animations(),
                |state, animations| state.set_animations(animations),
            )
            .subtitle(if state.system_reduces_motion() {
                "Your desktop asks for less motion, so Full plays as Reduced"
            } else {
                "Reduced plays slowly and settles sooner, Off keeps backgrounds still"
            }),
        )
        .push(
            choice_row(
                look,
                "Cursor effect",
                state.cursor_effect(),
                |state, effect| state.set_cursor_effect(effect),
            )
            .subtitle(cursor_effect_description(state.cursor_effect())),
        )
        .push(
            spin_row(
                look,
                "Cursor effect size (%)",
                state.cursor_effect_size(),
                SpinRange {
                    values: CURSOR_EFFECT_SIZES,
                    step: CURSOR_EFFECT_SIZE_STEP,
                },
                |state, size| state.set_cursor_effect_size(size),
                state.cursor_effect() != CursorEffect::None,
            )
            .subtitle("How far the effect reaches around the pointer"),
        )
        .push_maybe(uses_grid(state).then(|| {
            switch_row(
                look,
                "Grid bulges around the pointer",
                state.grid_bulge(),
                |state, on| state.set_grid_bulge(on),
            )
            .subtitle("The grid swells under the pointer, unless animations are reduced")
        }));
    for row in color_rows(state, look) {
        group = group.push(row);
    }
    group.into()
}

fn has_picture(state: &AppState) -> bool {
    !state.background_picture().is_empty()
}

fn animation_picker<'a>(state: &AppState, look: Look) -> Element<'a, Message> {
    let chosen = state.background_animation();
    picker_grid(
        BackgroundAnimation::ALL.iter().map(|&animation| {
            picker_card(
                look,
                Card {
                    preview: animation_scenery(state, look, animation),
                    preview_height: ANIMATION_PREVIEW_HEIGHT,
                    preview_background: Background::Color(look.window),
                    label: animation.label().to_owned(),
                    is_chosen: animation == chosen,
                    on_press: choosing(move |state| state.set_background_animation(animation)),
                    trailing: None,
                },
            )
        }),
        ANIMATIONS_PER_ROW,
    )
}

fn app_font_label(state: &AppState) -> String {
    match state.ui_font().as_str() {
        "" => THEMES_OWN_FONT.to_owned(),
        family => family.to_owned(),
    }
}

fn app_font_picker<'a>(state: &AppState, look: Look) -> Element<'a, Message> {
    let chosen = state.ui_font();
    let theme_font = Look::of_theme(state).font;
    let imported = state.imported_fonts();
    let built_in = std::iter::once((String::new(), theme_font, false))
        .chain(
            [("Archivo", ARCHIVO), ("JetBrains Mono", JETBRAINS_MONO)]
                .into_iter()
                .map(|(family, font)| (family.to_owned(), font, false)),
        )
        .chain(PRESET_FONTS.iter().map(|preset| {
            (
                preset.family.to_owned(),
                Font::with_name(preset.family),
                false,
            )
        }))
        .chain(imported.0.iter().map(|font| {
            (
                font.family.clone(),
                Font {
                    weight: style::weight_of(font.weight),
                    ..Font::with_name(lasting_name(&font.family))
                },
                true,
            )
        }));
    let mut cards: Vec<Element<'a, Message>> = built_in
        .map(|(family, font, is_imported)| {
            let label = if family.is_empty() {
                THEMES_OWN_FONT.to_owned()
            } else {
                family.clone()
            };
            let trailing = is_imported.then(|| {
                icon_button(
                    look,
                    Symbol::Trash,
                    "Remove this imported font",
                    Some(Message::Settings(SettingsMessage::RemoveFont(
                        family.clone(),
                    ))),
                )
            });
            let is_chosen = family == chosen;
            picker_card(
                look,
                Card {
                    preview: font_sample(font, look),
                    preview_height: FONT_PREVIEW_HEIGHT,
                    preview_background: Background::Color(look.window),
                    label,
                    is_chosen,
                    on_press: choosing(move |state| state.set_ui_font(family.as_str())),
                    trailing,
                },
            )
        })
        .collect();
    cards.push(adding_card(
        look,
        "Import font",
        FONT_PREVIEW_HEIGHT,
        Message::Settings(SettingsMessage::ImportFont),
    ));
    picker_grid(cards, CARDS_PER_ROW)
}

pub fn font_sample<'a>(font: Font, look: Look) -> Element<'a, Message> {
    container(
        column![
            text("Aa")
                .font(font)
                .size(look.sized(24.0))
                .color(look.text),
            text("Quick brown fox")
                .font(font)
                .size(look.sized(13.0))
                .color(look.dimmed)
                .wrapping(text::Wrapping::None),
        ]
        .spacing(2),
    )
    .padding([8, 10])
    .into()
}

fn accent_row<'a>(state: &AppState, look: Look) -> Element<'a, Message> {
    let typed = state.accent_color();
    let parsed = parse_color(&typed);
    let theme_accent = Look::of_theme(state).accent;
    let shown = parsed.unwrap_or(theme_accent);
    let input = text_input(&hex(theme_accent), &typed)
        .font(look.mono)
        .size(look.sized(13.0))
        .width(COLOR_INPUT_WIDTH)
        .padding([6, 10])
        .style(style::input(look, typed.is_empty() || parsed.is_some()))
        .on_input(|text| Message::Settings(SettingsMessage::AccentEdited(text)));
    let swatch = container(space().width(SWATCH_SIZE).height(SWATCH_SIZE)).style(move |_| {
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
    let mut accent_row = preference_row(look, "Accent colour")
        .subtitle("Leave empty for the theme's accent. Use #rrggbb or rgb(…)")
        .suffix(row![swatch, input].spacing(8).align_y(Alignment::Center));
    if !typed.is_empty() {
        accent_row = accent_row.suffix(icon_button(
            look,
            Symbol::Clear,
            "Use the theme's accent",
            Some(act(|state| state.set_accent_color(""))),
        ));
    }
    accent_row.into()
}

fn animation_scenery<'a>(
    state: &AppState,
    look: Look,
    animation: BackgroundAnimation,
) -> Element<'a, Message> {
    let own = backdrop_for_theme(state, chosen_option(state).choice(), &look);
    let still: Element<'a, Message> = match &own.still {
        Still::Gradient(glow) => gradient_backdrop(glow),
        Still::Nothing | Still::Grid | Still::LineField | Still::Scanlines => {
            space().width(Length::Fill).height(Length::Fill).into()
        }
    };
    let scene_and_palette = match animation {
        BackgroundAnimation::None => None,
        BackgroundAnimation::ThemeDefault => own.scene,
        animation => Scene::chosen(animation).map(|scene| {
            let theme_colors = theme_colors_for(state, scene);
            let picked = picked_colors(state);
            let colors: Vec<Option<Color>> = theme_colors
                .iter()
                .enumerate()
                .map(|(index, &theme_color)| {
                    let typed = picked.get(index).copied().flatten();
                    Some(
                        typed
                            .filter(|_| state.background_animation() == animation)
                            .unwrap_or(theme_color),
                    )
                })
                .collect();
            (scene, scene.palette(look.window, &colors))
        }),
    };
    match scene_and_palette {
        Some((scene, palette)) => gpu_or(scene.preview(palette, own.motion), still),
        None => still,
    }
}

fn uses_grid(state: &AppState) -> bool {
    state.theme() == Theme::Grid
        && state
            .imported_themes()
            .named(&state.imported_theme())
            .is_none()
}

fn cursor_effect_description(effect: CursorEffect) -> &'static str {
    match effect {
        CursorEffect::None => "Nothing follows the pointer",
        CursorEffect::Glow => "A soft light in your theme's accent follows the pointer",
        CursorEffect::Spotlight => "Dims the window outside a circle around the pointer",
        CursorEffect::Ring => "A thin ring in your theme's accent circles the pointer",
        CursorEffect::Ripples => "Clicks send out rings, unless animations are reduced",
        CursorEffect::Trail => {
            "Dots trail behind the moving pointer, unless animations are reduced"
        }
    }
}

fn chosen_option(state: &AppState) -> ThemeOption {
    let imported = state.imported_themes();
    match imported.named(&state.imported_theme()) {
        Some(theme) => ThemeOption::Imported(theme.name.clone()),
        None => ThemeOption::BuiltIn(state.theme()),
    }
}

fn gallery<'a>(state: &AppState, look: Look) -> Element<'a, Message> {
    let chosen = chosen_option(state);
    let imported = state.imported_themes();
    let mut cards: Vec<Element<'a, Message>> = Theme::ALL
        .iter()
        .copied()
        .map(ThemeOption::BuiltIn)
        .chain(
            imported
                .0
                .iter()
                .map(|theme| ThemeOption::Imported(theme.name.clone())),
        )
        .map(|option| {
            let is_chosen = option == chosen;
            theme_card(state, option, is_chosen, look)
        })
        .collect();
    cards.push(adding_card(
        look,
        "Import CSS theme",
        PREVIEW_HEIGHT,
        Message::Settings(SettingsMessage::ImportTheme),
    ));

    let mut rows = column![].spacing(CARD_SPACING);
    let mut cards = cards.into_iter().peekable();
    while cards.peek().is_some() {
        let mut gallery_row = row![].spacing(CARD_SPACING);
        for _ in 0..CARDS_PER_ROW {
            gallery_row = gallery_row.push(match cards.next() {
                Some(card) => card,
                None => space().width(Length::Fill).into(),
            });
        }
        rows = rows.push(gallery_row);
    }
    container(rows).padding(12).width(Length::Fill).into()
}

fn theme_card<'a>(
    state: &AppState,
    option: ThemeOption,
    is_chosen: bool,
    look: Look,
) -> Element<'a, Message> {
    let card_look = option.look(state);
    let backdrop = backdrop_for_theme(state, option.choice(), &card_look);
    let still: Element<'a, Message> = match &backdrop.still {
        Still::Gradient(glow) => gradient_backdrop(glow),
        Still::Nothing | Still::Grid | Still::LineField | Still::Scanlines => {
            space().width(Length::Fill).height(Length::Fill).into()
        }
    };
    let scenery: Element<'a, Message> = match backdrop.scene {
        Some((scene, palette)) => gpu_or(scene.preview(palette, backdrop.motion), still),
        None => still,
    };
    let sample = row![
        text("Aa")
            .font(Font {
                weight: Weight::Bold,
                ..card_look.font
            })
            .size(look.sized(18.0))
            .color(card_look.text),
        container(space().width(26).height(10)).style(move |_| container::Style {
            background: Some(Background::Color(card_look.accent)),
            border: Border {
                radius: 5.0.into(),
                ..Border::default()
            },
            ..container::Style::default()
        }),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    let preview = container(stack![scenery, container(sample).padding([8, 10])])
        .height(PREVIEW_HEIGHT)
        .width(Length::Fill)
        .clip(true)
        .style(move |_| container::Style {
            background: Some(Background::Color(card_look.window)),
            border: Border {
                radius: (look.radius - 2.0).max(0.0).into(),
                ..Border::default()
            },
            ..container::Style::default()
        });

    let mut label = row![
        text(option.name().to_owned())
            .font(look.font)
            .size(look.sized(13.0))
            .width(Length::Fill)
            .wrapping(text::Wrapping::None)
    ]
    .align_y(Alignment::Center)
    .height(LABEL_HEIGHT);
    if let ThemeOption::Imported(name) = &option {
        label = label.push(icon_button(
            look,
            Symbol::Trash,
            "Remove this imported theme",
            Some(Message::Settings(SettingsMessage::RemoveTheme(
                name.clone(),
            ))),
        ));
    } else if is_chosen {
        label = label.push(icon::tinted(Symbol::Check, look.accent_text));
    }

    button(column![preview, container(label).padding([2, 6])])
        .padding(4)
        .width(Length::Fill)
        .on_press(option.apply())
        .style(card_button(look, is_chosen))
        .into()
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
                .size(look.sized(13.0))
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
            color_row.into()
        })
        .collect()
}
