use crate::backend;
use crate::state::AppState;
use adw::prelude::*;
use gtk::glib;

const PROPERTIES_THAT_DECIDE_ACTION: [&str; 6] = [
    "running",
    "stopping",
    "busy",
    "can-play",
    "roblox-installed",
    "rosemary-installed",
];
const ACTION_STYLE_CLASSES: [&str; 2] = ["suggested-action", "destructive-action"];

#[derive(Clone, Copy, PartialEq, Eq)]
enum PlayButtonAction {
    InstallRoblox,
    Play,
    Stop,
    Stopping,
}

impl PlayButtonAction {
    fn for_state(state: &AppState) -> Self {
        if state.running() && state.stopping() {
            Self::Stopping
        } else if state.running() {
            Self::Stop
        } else if !state.roblox_installed() {
            Self::InstallRoblox
        } else {
            Self::Play
        }
    }

    fn is_available(self, state: &AppState) -> bool {
        match self {
            Self::InstallRoblox => !state.busy(),
            Self::Play => state.can_play() && !state.busy(),
            Self::Stop => true,
            Self::Stopping => false,
        }
    }

    fn explanation_when_unavailable(self, state: &AppState) -> Option<&'static str> {
        match self {
            Self::Play if !state.can_play() => {
                Some("The selected runner isn't installed. Choose another one in Settings.")
            }
            _ => None,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::InstallRoblox => "Install Roblox",
            Self::Play => "Play",
            Self::Stop => "Stop",
            Self::Stopping => "Stopping…",
        }
    }

    fn icon_name(self) -> &'static str {
        match self {
            Self::InstallRoblox => "folder-download-symbolic",
            Self::Play => "media-playback-start-symbolic",
            Self::Stop | Self::Stopping => "media-playback-stop-symbolic",
        }
    }

    fn style_class(self) -> Option<&'static str> {
        match self {
            Self::InstallRoblox | Self::Play => Some("suggested-action"),
            Self::Stop => Some("destructive-action"),
            Self::Stopping => None,
        }
    }

    fn perform(self, state: &AppState) {
        match self {
            Self::InstallRoblox => backend::install_roblox(state),
            Self::Play => backend::play(state),
            Self::Stop => backend::stop(state),
            Self::Stopping => {}
        }
    }
}

pub fn play_button(state: &AppState) -> gtk::Button {
    let content = adw::ButtonContent::new();

    let button = gtk::Button::builder()
        .child(&content)
        .css_classes(["pill"])
        .halign(gtk::Align::Center)
        .margin_top(12)
        .build();

    button.connect_clicked(glib::clone!(
        #[strong]
        state,
        move |_| PlayButtonAction::for_state(&state).perform(&state)
    ));

    show_action(&button, &content, state);
    for property in PROPERTIES_THAT_DECIDE_ACTION {
        state.connect_notify_local(
            Some(property),
            glib::clone!(
                #[weak]
                button,
                #[weak]
                content,
                move |state, _| show_action(&button, &content, state)
            ),
        );
    }

    button
}

fn show_action(button: &gtk::Button, content: &adw::ButtonContent, state: &AppState) {
    let action = PlayButtonAction::for_state(state);

    content.set_label(action.label());
    content.set_icon_name(action.icon_name());

    for style_class in ACTION_STYLE_CLASSES {
        button.remove_css_class(style_class);
    }
    if let Some(style_class) = action.style_class() {
        button.add_css_class(style_class);
    }

    button.set_sensitive(action.is_available(state));
    button.set_tooltip_text(action.explanation_when_unavailable(state));
}
