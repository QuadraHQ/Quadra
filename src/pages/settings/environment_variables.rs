use crate::state::{AppState, EnvironmentVariable, EnvironmentVariables};
use crate::utils::scroll_into_view_once_shown;
use adw::prelude::*;
use gtk::glib;
use std::cell::RefCell;
use std::rc::Rc;

struct VariableRow {
    row: adw::EntryRow,
    enabled_switch: gtk::Switch,
}

type VariableRows = Rc<RefCell<Vec<VariableRow>>>;

pub fn environment_variables_group(state: &AppState) -> adw::PreferencesGroup {
    let add_button = gtk::Button::builder()
        .icon_name("list-add-symbolic")
        .tooltip_text("Add a variable")
        .css_classes(["flat"])
        .valign(gtk::Align::Center)
        .build();

    let group = adw::PreferencesGroup::builder()
        .title("Environment variables")
        .description(
            "Extra variables passed to Rosemary when Roblox starts, written as NAME=value.",
        )
        .header_suffix(&add_button)
        .build();

    let rows = VariableRows::default();
    for variable in &state.environment_variables().0 {
        add_variable_row(&group, &rows, state, variable);
    }

    add_button.connect_clicked(glib::clone!(
        #[weak]
        group,
        #[strong]
        rows,
        #[strong]
        state,
        move |_| {
            let new_variable = EnvironmentVariable {
                enabled: true,
                ..Default::default()
            };
            let row = add_variable_row(&group, &rows, &state, &new_variable);
            scroll_into_view_once_shown(&row);
            row.grab_focus();
            save_variables(&state, &rows);
        }
    ));

    group
}

fn add_variable_row(
    group: &adw::PreferencesGroup,
    rows: &VariableRows,
    state: &AppState,
    variable: &EnvironmentVariable,
) -> adw::EntryRow {
    let enabled_switch = gtk::Switch::builder()
        .active(variable.enabled)
        .tooltip_text("Pass this variable to Rosemary")
        .valign(gtk::Align::Center)
        .build();
    let remove_button = gtk::Button::builder()
        .icon_name("user-trash-symbolic")
        .tooltip_text("Remove this variable")
        .css_classes(["flat"])
        .valign(gtk::Align::Center)
        .build();

    let row = adw::EntryRow::builder()
        .title("NAME=value")
        .text(variable_as_text(variable))
        .build();
    row.add_suffix(&enabled_switch);
    row.add_suffix(&remove_button);
    group.add(&row);

    rows.borrow_mut().push(VariableRow {
        row: row.clone(),
        enabled_switch: enabled_switch.clone(),
    });

    row.connect_changed(glib::clone!(
        #[strong]
        state,
        #[strong]
        rows,
        move |_| save_variables(&state, &rows)
    ));
    enabled_switch.connect_active_notify(glib::clone!(
        #[strong]
        state,
        #[strong]
        rows,
        move |_| save_variables(&state, &rows)
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
            rows.borrow_mut()
                .retain(|variable_row| variable_row.row != row);
            group.remove(&row);
            save_variables(&state, &rows);
        }
    ));

    row
}

fn save_variables(state: &AppState, rows: &VariableRows) {
    let variables = rows
        .borrow()
        .iter()
        .map(|variable_row| {
            variable_from_text(
                &variable_row.row.text(),
                variable_row.enabled_switch.is_active(),
            )
        })
        .collect();
    state.set_environment_variables(EnvironmentVariables(variables));
}

fn variable_as_text(variable: &EnvironmentVariable) -> String {
    if variable.name.is_empty() && variable.value.is_empty() {
        String::new()
    } else {
        format!("{}={}", variable.name, variable.value)
    }
}

fn variable_from_text(text: &str, enabled: bool) -> EnvironmentVariable {
    let (name, value) = text.split_once('=').unwrap_or((text, ""));
    EnvironmentVariable {
        name: name.trim().to_owned(),
        value: value.to_owned(),
        enabled,
    }
}
