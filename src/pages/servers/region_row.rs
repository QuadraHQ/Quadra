use crate::components::preferences::preference_row;
use crate::components::style::{self, Look};
use crate::quadra::{Message, act};
use crate::state::{AppState, Region};
use iced::Element;
use iced::widget::pick_list;
use std::fmt;

const NO_PREFERENCE_LABEL: &str = "No preference";

#[derive(Debug, Clone, PartialEq, Eq)]
struct RegionChoice {
    key: String,
    label: String,
}

impl fmt::Display for RegionChoice {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.label)
    }
}

pub fn preferred_region_row<'a>(state: &AppState, look: Look) -> Element<'a, Message> {
    let preferred_region = state.preferred_region();
    let choices = region_choices(&state.regions().0, &preferred_region);
    let selected = choices
        .iter()
        .find(|choice| choice.key == preferred_region)
        .cloned();

    let picker = pick_list(choices, selected, |choice: RegionChoice| {
        act(move |state| state.set_preferred_region(choice.key.clone()))
    })
    .font(look.font)
    .text_size(14)
    .style(style::choice(look))
    .menu_style(style::menu(look));

    preference_row(look, "Preferred region")
        .suffix(picker)
        .into()
}

fn region_choices(regions: &[Region], preferred_region: &str) -> Vec<RegionChoice> {
    let mut choices = vec![RegionChoice {
        key: String::new(),
        label: NO_PREFERENCE_LABEL.to_owned(),
    }];
    choices.extend(regions.iter().map(|region| RegionChoice {
        key: region.key(),
        label: region.label(),
    }));

    let preferred_region_is_unlisted = !preferred_region.is_empty()
        && !choices.iter().any(|choice| choice.key == preferred_region);
    if preferred_region_is_unlisted {
        choices.push(RegionChoice {
            key: preferred_region.to_owned(),
            label: preferred_region.to_owned(),
        });
    }

    choices
}
