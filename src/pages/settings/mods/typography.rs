use crate::components::file_row::{ChosenPath, Pick, file_row};
use crate::components::picker::{Card, Opener, choosing, picker_card, picker_grid, picker_row};
use crate::components::preferences::{preference_group, searchable};
use crate::components::style::Look;
use crate::pages::settings::appearance::font_sample;
use crate::quadra::Message;
use crate::state::{AppState, Choice, FontStyle, Picker};
use iced::widget::text;
use iced::{Background, Element, Font};

const FONT_FILES: &[Pick] = &[Pick::File(&["ttf", "otf"])];
const FONT_PREVIEW_HEIGHT: f32 = 62.0;
const FONTS_PER_ROW: usize = 3;

pub fn typography_group<'a>(state: &AppState, look: Look) -> Element<'a, Message> {
    let style = state.font_style();
    let custom_row = (style == FontStyle::Custom)
        .then(|| file_row(state, look, ChosenPath::Font, "Custom font", FONT_FILES));

    preference_group(look)
        .title("Typography")
        .push(picker_row(
            state,
            look,
            Opener {
                picker: Picker::GameFont,
                title: "Font",
                subtitle: "Replaces the text font inside the game",
                shown: text(style.label())
                    .font(font_of(style, look))
                    .size(14)
                    .into(),
            },
        ))
        .push_maybe(
            (state.open_picker() == Picker::GameFont)
                .then(|| searchable(game_font_picker(style, look), "Font picker typeface")),
        )
        .push_maybe(custom_row)
        .into()
}

fn font_of(style: FontStyle, look: Look) -> Font {
    style
        .preset()
        .map_or(look.font, |preset| Font::with_name(preset.family))
}

fn game_font_picker<'a>(chosen: FontStyle, look: Look) -> Element<'a, Message> {
    picker_grid(
        FontStyle::ALL.iter().map(|&style| {
            picker_card(
                look,
                Card {
                    preview: font_sample(font_of(style, look), look),
                    preview_height: FONT_PREVIEW_HEIGHT,
                    preview_background: Background::Color(look.window),
                    label: style.label().to_owned(),
                    is_chosen: style == chosen,
                    on_press: choosing(move |state| state.set_font_style(style)),
                    trailing: None,
                },
            )
        }),
        FONTS_PER_ROW,
    )
}
