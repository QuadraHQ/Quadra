use crate::state::AppState;
use adw::prelude::*;
use gtk::glib;

const PROPERTIES_THAT_DECIDE_ACTION: [&str; 2] = ["running", "can-play"];
const ACTION_STYLE_CLASSES: [&str; 2] = ["suggested-action", "destructive-action"];

#[derive(Clone, Copy)]
enum PlayButtonAction {
    InstallRoblox,
    Play,
    Stop,
}

impl PlayButtonAction {
    fn for_state(state: &AppState) -> Self {
        if state.running() {
            Self::Stop
        } else if state.can_play() {
            Self::Play
        } else {
            Self::InstallRoblox
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::InstallRoblox => "Install Roblox",
            Self::Play => "Play",
            Self::Stop => "Stop",
        }
    }

    fn icon_name(self) -> &'static str {
        match self {
            Self::InstallRoblox => "folder-download-symbolic",
            Self::Play => "media-playback-start-symbolic",
            Self::Stop => "media-playback-stop-symbolic",
        }
    }

    fn style_class(self) -> &'static str {
        match self {
            Self::InstallRoblox | Self::Play => "suggested-action",
            Self::Stop => "destructive-action",
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
        move |_| match PlayButtonAction::for_state(&state) {
            PlayButtonAction::InstallRoblox => println!("install roblox clicked"),
            PlayButtonAction::Play => println!("play clicked"),
            PlayButtonAction::Stop => println!("stop clicked"),
        }
    ));

    show_action(&button, &content, PlayButtonAction::for_state(state));
    for property in PROPERTIES_THAT_DECIDE_ACTION {
        state.connect_notify_local(
            Some(property),
            glib::clone!(
                #[weak]
                button,
                #[weak]
                content,
                move |state, _| show_action(&button, &content, PlayButtonAction::for_state(state))
            ),
        );
    }

    button
}

fn show_action(button: &gtk::Button, content: &adw::ButtonContent, action: PlayButtonAction) {
    content.set_label(action.label());
    content.set_icon_name(action.icon_name());

    for style_class in ACTION_STYLE_CLASSES {
        button.remove_css_class(style_class);
    }
    button.add_css_class(action.style_class());
}
