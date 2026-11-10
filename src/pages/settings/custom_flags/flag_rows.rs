use crate::components::ignored_flag_icon::{ignored_flag_icon, show_if_ignored};
use crate::pages::settings::fast_flag_allowlist::is_listed;
use crate::state::{AppState, FastFlagValue, FastFlags};
use crate::utils::scroll_into_view_once_shown;
use adw::prelude::*;
use gtk::glib;
use std::cell::RefCell;
use std::rc::Rc;

pub type FlagRows = Rc<RefCell<Vec<adw::EntryRow>>>;

pub fn show_custom_flags(group: &adw::PreferencesGroup, rows: &FlagRows, state: &AppState) {
    for row in rows.borrow_mut().drain(..) {
        group.remove(&row);
    }
    for (name, value) in &state.fast_flags().0 {
        if !is_listed(name) {
            add_row(group, rows, state, &format!("{name}={}", value.as_text()));
        }
    }
}

pub fn add_flag(group: &adw::PreferencesGroup, rows: &FlagRows, state: &AppState) {
    let row = add_row(group, rows, state, "");
    scroll_into_view_once_shown(&row);
    row.grab_focus();
}

fn add_row(
    group: &adw::PreferencesGroup,
    rows: &FlagRows,
    state: &AppState,
    text: &str,
) -> adw::EntryRow {
    let ignored_icon = ignored_flag_icon();
    let remove_button = gtk::Button::builder()
        .icon_name("user-trash-symbolic")
        .tooltip_text("Remove this flag")
        .css_classes(["flat"])
        .valign(gtk::Align::Center)
        .build();

    let row = adw::EntryRow::builder()
        .title("NAME=value")
        .text(text)
        .build();
    row.add_prefix(&ignored_icon);
    row.add_suffix(&remove_button);
    group.add(&row);
    rows.borrow_mut().push(row.clone());
    show_if_ignored(&ignored_icon, flag_of(text).0);

    row.connect_changed(glib::clone!(
        #[strong]
        state,
        #[strong]
        rows,
        move |row| {
            show_if_ignored(&ignored_icon, flag_of(&row.text()).0);
            save_flags(&state, &rows);
        }
    ));
    remove_button.connect_clicked(glib::clone!(
        #[weak]
        group,
        #[weak]
        row,
        #[strong]
        state,
        #[strong]
        rows,
        move |_| {
            rows.borrow_mut().retain(|other| *other != row);
            group.remove(&row);
            save_flags(&state, &rows);
        }
    ));

    row
}

fn save_flags(state: &AppState, rows: &FlagRows) {
    let mut flags: std::collections::BTreeMap<String, FastFlagValue> = state
        .fast_flags()
        .0
        .into_iter()
        .filter(|(name, _)| is_listed(name))
        .collect();

    for row in rows.borrow().iter() {
        let text = row.text();
        let (name, value) = flag_of(&text);
        if !name.is_empty() && !is_listed(name) {
            flags.insert(name.to_owned(), FastFlagValue::parse(value));
        }
    }
    state.set_fast_flags(FastFlags(flags));
}

fn flag_of(text: &str) -> (&str, &str) {
    let (name, value) = text.split_once('=').unwrap_or((text, ""));
    (name.trim(), value)
}
