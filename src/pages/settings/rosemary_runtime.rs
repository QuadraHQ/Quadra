use super::bound_rows::{ChoiceAction, named_choice_row};
use crate::backend::{self, QUADRA_ROSEMARY_VERSION};
use crate::state::{AppState, QUADRA_PREFIX, QUADRA_RUNNER};
use adw::prelude::*;
use gtk::glib;

const PROPERTIES_THAT_DECIDE_ROSEMARY_INSTALL_ROW: [&str; 4] =
    ["rosemary-installed", "runner", "first-launch", "busy"];
const PROPERTIES_THAT_DECIDE_RUNNER_DESCRIPTION: [&str; 2] = ["rosemary-installed", "runner"];
const PROPERTIES_THAT_DECIDE_PREFIX_STATUS: [&str; 3] =
    ["darwin-base-installed", "rosemary-installed", "busy"];

pub fn rosemary_runtime_group(state: &AppState) -> adw::PreferencesGroup {
    let choose_custom_runner = ChoiceAction {
        label: "Choose a runner file…",
        perform: Box::new(glib::clone!(
            #[weak]
            state,
            move |row| choose_custom_runner(&state, row)
        )),
    };
    let runner_row = named_choice_row(
        state,
        "runner",
        "available-runners",
        "Runner",
        Some(choose_custom_runner),
    );
    runner_row.set_subtitle(&runner_description(state));
    for property in PROPERTIES_THAT_DECIDE_RUNNER_DESCRIPTION {
        state.connect_notify_local(
            Some(property),
            glib::clone!(
                #[weak]
                runner_row,
                move |state, _| runner_row.set_subtitle(&runner_description(state))
            ),
        );
    }

    let prefix_row = named_choice_row(state, "prefix", "available-prefixes", "Prefix", None);
    prefix_row.set_subtitle("The Darwin filesystem Roblox runs in.");

    let group = adw::PreferencesGroup::builder()
        .title("Rosemary")
        .description("Which Rosemary runs Roblox, and where.")
        .build();
    group.add(&rosemary_install_row(state));
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
            PrefixAction::InstallDarwinBase => backend::install_darwin_base(&state),
            PrefixAction::RepairPrefix => backend::repair_prefix(&state),
        }
    ));

    show_prefix_status(&row, &status_icon, &action_button, state);
    for property in PROPERTIES_THAT_DECIDE_PREFIX_STATUS {
        state.connect_notify_local(
            Some(property),
            glib::clone!(
                #[weak]
                row,
                #[weak]
                status_icon,
                #[weak]
                action_button,
                move |state, _| show_prefix_status(&row, &status_icon, &action_button, state)
            ),
        );
    }

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
    action_button.set_sensitive(state.rosemary_installed() && !state.busy());

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

fn rosemary_install_row(state: &AppState) -> adw::ActionRow {
    let install_button = gtk::Button::builder()
        .label("Install")
        .css_classes(["suggested-action"])
        .valign(gtk::Align::Center)
        .build();
    install_button.connect_clicked(glib::clone!(
        #[strong]
        state,
        move |_| backend::install_rosemary(&state)
    ));

    let row = adw::ActionRow::builder()
        .title("Rosemary isn't installed")
        .subtitle(format!(
            "Install Quadra's Rosemary {QUADRA_ROSEMARY_VERSION} to run Roblox."
        ))
        .build();
    row.add_suffix(&install_button);

    show_rosemary_install_row(&row, &install_button, state);
    for property in PROPERTIES_THAT_DECIDE_ROSEMARY_INSTALL_ROW {
        state.connect_notify_local(
            Some(property),
            glib::clone!(
                #[weak]
                row,
                #[weak]
                install_button,
                move |state, _| show_rosemary_install_row(&row, &install_button, state)
            ),
        );
    }

    row
}

fn show_rosemary_install_row(row: &adw::ActionRow, install_button: &gtk::Button, state: &AppState) {
    row.set_visible(quadra_rosemary_is_selected_but_missing(state) && !state.first_launch());
    install_button.set_sensitive(!state.busy());
}

fn choose_custom_runner(state: &AppState, row: &adw::ComboRow) {
    let dialog = gtk::FileDialog::builder()
        .title("Choose a Rosemary runner")
        .accept_label("Use as runner")
        .modal(true)
        .build();
    let window = row.root().and_downcast::<gtk::Window>();

    dialog.open(
        window.as_ref(),
        gtk::gio::Cancellable::NONE,
        glib::clone!(
            #[weak]
            state,
            move |chosen_file| {
                if let Some(executable) = chosen_file.ok().and_then(|file| file.path()) {
                    backend::use_custom_runner(&state, &executable);
                }
            }
        ),
    );
}

fn quadra_rosemary_is_selected_but_missing(state: &AppState) -> bool {
    state.runner() == QUADRA_RUNNER && !state.rosemary_installed()
}

fn runner_description(state: &AppState) -> String {
    if quadra_rosemary_is_selected_but_missing(state) {
        format!("Quadra's Rosemary {QUADRA_ROSEMARY_VERSION} will be installed when you play.")
    } else {
        "The Rosemary build that runs Roblox.".to_owned()
    }
}
