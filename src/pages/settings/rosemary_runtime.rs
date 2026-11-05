use super::bound_rows::named_choice_row;
use crate::state::{AppState, QUADRA_PREFIX};
use adw::prelude::*;
use gtk::glib;

pub fn rosemary_runtime_group(state: &AppState) -> adw::PreferencesGroup {
    let runner_row = named_choice_row(state, "runner", "available-runners", "Runner");
    runner_row.set_subtitle("The Rosemary build that runs Roblox.");

    let prefix_row = named_choice_row(state, "prefix", "available-prefixes", "Prefix");
    prefix_row.set_subtitle("The Darwin filesystem Roblox runs in.");

    let group = adw::PreferencesGroup::builder()
        .title("Rosemary")
        .description("Which Rosemary runs Roblox, and where.")
        .build();
    group.add(&runner_row);
    group.add(&prefix_row);
    group.add(&prefix_status_row(state));

    group
}

#[derive(Clone, Copy)]
enum PrefixAction {
    InstallDarwinBase,
    RepairPrefix,
}

impl PrefixAction {
    fn for_state(state: &AppState) -> Self {
        if state.darwin_base_installed() {
            Self::RepairPrefix
        } else {
            Self::InstallDarwinBase
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::InstallDarwinBase => "Install",
            Self::RepairPrefix => "Repair",
        }
    }
}

fn prefix_status_row(state: &AppState) -> adw::ActionRow {
    let status_icon = gtk::Image::new();
    let action_button = gtk::Button::builder().valign(gtk::Align::Center).build();

    let row = adw::ActionRow::builder().title("Prefix status").build();
    row.add_prefix(&status_icon);
    row.add_suffix(&action_button);

    action_button.connect_clicked(glib::clone!(
        #[strong]
        state,
        move |_| match PrefixAction::for_state(&state) {
            PrefixAction::InstallDarwinBase => println!("install darwin base clicked"),
            PrefixAction::RepairPrefix => println!("repair prefix clicked"),
        }
    ));

    show_prefix_status(&row, &status_icon, &action_button, state);
    state.connect_darwin_base_installed_notify(glib::clone!(
        #[weak]
        row,
        #[weak]
        status_icon,
        #[weak]
        action_button,
        move |state| show_prefix_status(&row, &status_icon, &action_button, state)
    ));

    state
        .bind_property("prefix", &row, "sensitive")
        .transform_to(|_, prefix: String| Some(prefix == QUADRA_PREFIX))
        .sync_create()
        .build();

    row
}

fn show_prefix_status(
    row: &adw::ActionRow,
    status_icon: &gtk::Image,
    action_button: &gtk::Button,
    state: &AppState,
) {
    action_button.set_label(PrefixAction::for_state(state).label());

    if state.darwin_base_installed() {
        row.set_subtitle("Darwin base installed");
        status_icon.set_icon_name(Some("object-select-symbolic"));
        status_icon.remove_css_class("warning");
        status_icon.add_css_class("success");
    } else {
        row.set_subtitle("Darwin base not installed");
        status_icon.set_icon_name(Some("dialog-warning-symbolic"));
        status_icon.remove_css_class("success");
        status_icon.add_css_class("warning");
    }
}
