use crate::state::{AppState, Choice};
use adw::prelude::*;
use gtk::glib;
use std::cell::Cell;
use std::rc::Rc;

pub fn choice_row<T>(state: &AppState, property: &str, title: &str) -> adw::ComboRow
where
    T: Choice + glib::value::ValueType + Into<glib::Value>,
{
    let row = adw::ComboRow::builder()
        .title(title)
        .model(&gtk::StringList::new(&T::labels()))
        .build();

    state
        .bind_property(property, &row, "selected")
        .transform_to(|_, choice: T| Some(choice.position()))
        .transform_from(|_, position: u32| T::at_position(position))
        .bidirectional()
        .sync_create()
        .build();

    row
}

pub fn switch_row(state: &AppState, property: &str, title: &str) -> adw::SwitchRow {
    let row = adw::SwitchRow::builder().title(title).build();

    state
        .bind_property(property, &row, "active")
        .bidirectional()
        .sync_create()
        .build();

    row
}

const NOTHING_CHOSEN_LABEL: &str = "None";

pub struct ChoiceAction {
    pub label: &'static str,
    pub perform: Box<dyn Fn(&adw::ComboRow)>,
}

enum ChoiceEntry<'a> {
    Name(&'a str),
    NothingChosen,
    Action,
}

fn choice_entries(names: &[String], has_action: bool) -> Vec<ChoiceEntry<'_>> {
    let mut entries: Vec<ChoiceEntry> = names.iter().map(|name| ChoiceEntry::Name(name)).collect();
    if has_action {
        if entries.is_empty() {
            entries.push(ChoiceEntry::NothingChosen);
        }
        entries.push(ChoiceEntry::Action);
    }
    entries
}

pub fn named_choice_row(
    state: &AppState,
    selected_property: &'static str,
    options_property: &'static str,
    title: &str,
    action: Option<ChoiceAction>,
) -> adw::ComboRow {
    let options = gtk::StringList::new(&[]);
    let row = adw::ComboRow::builder()
        .title(title)
        .model(&options)
        .build();
    let is_updating_row = Rc::new(Cell::new(false));
    let action_label = action.as_ref().map(|action| action.label);

    show_choices(
        &row,
        &options,
        state,
        selected_property,
        options_property,
        action_label,
        &is_updating_row,
    );

    row.connect_selected_notify(glib::clone!(
        #[weak]
        state,
        #[strong]
        is_updating_row,
        move |row| {
            if is_updating_row.get() {
                return;
            }
            let names = state.property::<Vec<String>>(options_property);
            let entries = choice_entries(&names, action.is_some());

            match (entries.get(row.selected() as usize), &action) {
                (Some(ChoiceEntry::Name(name)), _) => state.set_property(selected_property, *name),
                (Some(ChoiceEntry::Action), Some(action)) => {
                    let current_choice = state.property::<String>(selected_property);
                    select_choice(row, &entries, &current_choice, &is_updating_row);
                    (action.perform)(row);
                }
                _ => {}
            }
        }
    ));

    state.connect_notify_local(
        Some(selected_property),
        glib::clone!(
            #[weak]
            row,
            #[strong]
            is_updating_row,
            move |state, _| select_choice(
                &row,
                &choice_entries(
                    &state.property::<Vec<String>>(options_property),
                    action_label.is_some()
                ),
                &state.property::<String>(selected_property),
                &is_updating_row
            )
        ),
    );

    state.connect_notify_local(
        Some(options_property),
        glib::clone!(
            #[weak]
            row,
            #[weak]
            options,
            #[strong]
            is_updating_row,
            move |state, _| show_choices(
                &row,
                &options,
                state,
                selected_property,
                options_property,
                action_label,
                &is_updating_row
            )
        ),
    );

    row
}

fn show_choices(
    row: &adw::ComboRow,
    options: &gtk::StringList,
    state: &AppState,
    selected_property: &str,
    options_property: &str,
    action_label: Option<&str>,
    is_updating_row: &Cell<bool>,
) {
    let names = state.property::<Vec<String>>(options_property);
    let saved_choice = state.property::<String>(selected_property);
    let entries = choice_entries(&names, action_label.is_some());

    let labels: Vec<&str> = entries
        .iter()
        .map(|entry| match entry {
            ChoiceEntry::Name(name) => name,
            ChoiceEntry::NothingChosen => NOTHING_CHOSEN_LABEL,
            ChoiceEntry::Action => action_label.unwrap_or_default(),
        })
        .collect();
    is_updating_row.set(true);
    options.splice(0, options.n_items(), &labels);
    is_updating_row.set(false);

    if names.contains(&saved_choice) {
        select_choice(row, &entries, &saved_choice, is_updating_row);
    } else if let Some(first_name) = names.first() {
        select_choice(row, &entries, first_name, is_updating_row);
        state.set_property(selected_property, first_name);
    } else {
        select_choice(row, &entries, &saved_choice, is_updating_row);
    }
}

fn select_choice(
    row: &adw::ComboRow,
    entries: &[ChoiceEntry],
    choice: &str,
    is_updating_row: &Cell<bool>,
) {
    let chosen_name_position = entries
        .iter()
        .position(|entry| matches!(entry, ChoiceEntry::Name(name) if *name == choice));
    let nothing_chosen_position = entries
        .iter()
        .position(|entry| matches!(entry, ChoiceEntry::NothingChosen));
    let position = chosen_name_position
        .or(nothing_chosen_position)
        .map_or(gtk::INVALID_LIST_POSITION, |position| position as u32);

    is_updating_row.set(true);
    row.set_selected(position);
    is_updating_row.set(false);
}
