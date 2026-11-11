use crate::state::{AppState, Region};
use adw::prelude::*;
use gtk::glib;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const NO_PREFERENCE_LABEL: &str = "No preference";

struct RegionChoice {
    key: String,
    label: String,
}

#[derive(Clone, Default)]
struct ShownChoices {
    choices: Rc<RefCell<Vec<RegionChoice>>>,
    is_updating_row: Rc<Cell<bool>>,
}

pub fn preferred_region_row(state: &AppState) -> adw::ComboRow {
    let options = gtk::StringList::new(&[]);
    let row = adw::ComboRow::builder()
        .title("Preferred region")
        .model(&options)
        .build();
    let shown = ShownChoices::default();

    row.connect_selected_notify(glib::clone!(
        #[weak]
        state,
        #[strong]
        shown,
        move |row| {
            if shown.is_updating_row.get() {
                return;
            }
            let chosen_key = shown
                .choices
                .borrow()
                .get(row.selected() as usize)
                .map(|choice| choice.key.clone());
            if let Some(chosen_key) = chosen_key {
                state.set_preferred_region(chosen_key);
            }
        }
    ));

    state.follow_properties(
        &["regions", "preferred-region"],
        glib::clone!(
            #[weak]
            row,
            #[weak]
            options,
            #[strong]
            shown,
            move |state| show_choices(&row, &options, &shown, state)
        ),
    );

    row
}

fn show_choices(
    row: &adw::ComboRow,
    options: &gtk::StringList,
    shown: &ShownChoices,
    state: &AppState,
) {
    let preferred_region = state.preferred_region();
    let choices = region_choices(&state.regions().0, &preferred_region);
    let labels: Vec<&str> = choices.iter().map(|choice| choice.label.as_str()).collect();
    let preferred_position = choices
        .iter()
        .position(|choice| choice.key == preferred_region)
        .unwrap_or_default();

    shown.is_updating_row.set(true);
    options.splice(0, options.n_items(), &labels);
    row.set_selected(preferred_position as u32);
    shown.is_updating_row.set(false);

    shown.choices.replace(choices);
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
