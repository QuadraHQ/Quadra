use super::clock::every_second_while_shown;
use super::current_server::{CurrentServer, session_time};
use super::fade::Fade;
use super::panel::{PanelChange, PanelListener};
use crate::state::{AppState, PanelId};
use gtk::glib;
use gtk::prelude::*;
use std::rc::Rc;

const PROPERTIES_THAT_DECIDE_STATUS: [&str; 2] = ["in-game", "server-history"];

pub struct Dock {
    pub widget: gtk::Box,
    pub fade: Rc<Fade>,
    toggles: Vec<(PanelId, gtk::ToggleButton)>,
}

impl Dock {
    pub fn new(state: &AppState, listener: &PanelListener, close_overlay: Rc<dyn Fn()>) -> Self {
        let game_name = gtk::Label::builder()
            .xalign(0.0)
            .max_width_chars(28)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .css_classes(["game-name"])
            .build();
        let game_status = gtk::Label::builder()
            .xalign(0.0)
            .css_classes(["game-status", "dim-label"])
            .build();
        let game = gtk::Box::new(gtk::Orientation::Vertical, 0);
        game.set_margin_end(8);
        game.append(&game_name);
        game.append(&game_status);
        show_game(state, &game_name, &game_status);
        for property in PROPERTIES_THAT_DECIDE_STATUS {
            state.connect_notify_local(
                Some(property),
                glib::clone!(
                    #[weak]
                    game_name,
                    #[weak]
                    game_status,
                    move |state, _| show_game(state, &game_name, &game_status)
                ),
            );
        }
        every_second_while_shown(&game_status, {
            let state = state.clone();
            let game_name = game_name.clone();
            move |game_status| show_game(&state, &game_name, game_status)
        });

        let widget = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        widget.add_css_class("overlay-dock");
        widget.set_halign(gtk::Align::Center);
        widget.set_valign(gtk::Align::End);
        widget.set_margin_bottom(28);
        widget.append(&game);
        widget.append(&gtk::Separator::new(gtk::Orientation::Vertical));

        let toggles: Vec<(PanelId, gtk::ToggleButton)> = PanelId::ALL
            .into_iter()
            .map(|id| {
                let toggle = gtk::ToggleButton::builder()
                    .icon_name(id.icon_name())
                    .tooltip_text(id.title())
                    .css_classes(["flat", "circular"])
                    .build();
                toggle.connect_toggled({
                    let listener = listener.clone();
                    move |toggle| listener(id, PanelChange::Shown(toggle.is_active()))
                });
                widget.append(&toggle);
                (id, toggle)
            })
            .collect();

        widget.append(&gtk::Separator::new(gtk::Orientation::Vertical));
        let close = gtk::Button::builder()
            .icon_name("window-close-symbolic")
            .tooltip_text("Close the overlay")
            .css_classes(["flat", "circular"])
            .build();
        close.connect_clicked(move |_| close_overlay());
        widget.append(&close);

        let fade = Fade::new(&widget);
        Self {
            widget,
            fade,
            toggles,
        }
    }

    pub fn set_shown(&self, id: PanelId, shown: bool) {
        if let Some((_, toggle)) = self.toggles.iter().find(|(toggle_id, _)| *toggle_id == id) {
            toggle.set_active(shown);
        }
    }
}

fn show_game(state: &AppState, name: &gtk::Label, status: &gtk::Label) {
    match CurrentServer::of(state) {
        Some(server) => {
            name.set_label(&server.game_name());
            status.set_label(&format!(
                "Session {} · Server {}",
                session_time(state),
                server.time_on_server()
            ));
        }
        None => {
            name.set_label("Roblox");
            status.set_label(&format!("Session {} · Not in a game", session_time(state)));
        }
    }
}
