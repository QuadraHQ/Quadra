use crate::state::{AppState, Choice};
use adw::prelude::*;
use gtk::glib;

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

pub fn named_choice_row(
    state: &AppState,
    selected_property: &'static str,
    options_property: &'static str,
    title: &str,
) -> adw::ComboRow {
    let options = gtk::StringList::new(&[]);
    let row = adw::ComboRow::builder()
        .title(title)
        .model(&options)
        .build();

    show_options(&options, &state.property::<Vec<String>>(options_property));
    select_option(&row, &options, &state.property::<String>(selected_property));

    row.connect_selected_item_notify(glib::clone!(
        #[weak]
        state,
        move |row| {
            if let Some(option) = row.selected_item().and_downcast::<gtk::StringObject>() {
                state.set_property(selected_property, option.string());
            }
        }
    ));

    state.connect_notify_local(
        Some(selected_property),
        glib::clone!(
            #[weak]
            row,
            #[weak]
            options,
            move |state, _| select_option(
                &row,
                &options,
                &state.property::<String>(selected_property)
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
            move |state, _| {
                let selected_before_update = state.property::<String>(selected_property);
                show_options(&options, &state.property::<Vec<String>>(options_property));
                select_option(&row, &options, &selected_before_update);
            }
        ),
    );

    row
}

fn show_options(options: &gtk::StringList, names: &[String]) {
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    options.splice(0, options.n_items(), &names);
}

fn select_option(row: &adw::ComboRow, options: &gtk::StringList, name: &str) {
    let position =
        (0..options.n_items()).find(|&position| options.string(position).as_deref() == Some(name));
    if let Some(position) = position {
        row.set_selected(position);
    }
}
