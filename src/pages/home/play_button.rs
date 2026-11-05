use crate::components::play_action::{PROPERTIES_THAT_DECIDE_PLAY_ACTION, PlayAction};
use crate::state::AppState;
use adw::prelude::*;
use gtk::glib;

const ACTION_STYLE_CLASSES: [&str; 2] = ["suggested-action", "destructive-action"];

fn style_class(action: PlayAction) -> Option<&'static str> {
    match action {
        PlayAction::InstallRoblox | PlayAction::Play => Some("suggested-action"),
        PlayAction::Stop => Some("destructive-action"),
        PlayAction::Stopping => None,
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
        move |_| PlayAction::for_state(&state).perform(&state)
    ));

    show_action(&button, &content, state);
    for property in PROPERTIES_THAT_DECIDE_PLAY_ACTION {
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
    let action = PlayAction::for_state(state);

    content.set_label(action.label());
    content.set_icon_name(action.icon_name());

    for style_class in ACTION_STYLE_CLASSES {
        button.remove_css_class(style_class);
    }
    if let Some(style_class) = style_class(action) {
        button.add_css_class(style_class);
    }

    button.set_sensitive(action.is_available(state));
    button.set_tooltip_text(action.explanation_when_unavailable(state));
}
