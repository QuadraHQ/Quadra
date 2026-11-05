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
