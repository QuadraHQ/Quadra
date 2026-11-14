use crate::backend;
use crate::components::copy_button::copy_on_click;
use crate::components::overlay::clock::every_second_while_shown;
use crate::components::overlay::current_server::{CurrentServer, session_time};
use crate::state::AppState;
use gtk::glib;
use gtk::prelude::*;

const NOT_AVAILABLE: &str = "—";
const PROPERTIES_THAT_DECIDE_CONTENT: [&str; 2] = ["in-game", "server-history"];

struct Fields {
    game: gtk::Label,
    place: gtk::Label,
    server: gtk::Label,
    address: gtk::Label,
    location: gtk::Label,
    session: gtk::Label,
    time_on_server: gtk::Label,
}

pub fn server_content(state: &AppState) -> gtk::Widget {
    let fields = Fields {
        game: value_label(),
        place: value_label(),
        server: value_label(),
        address: value_label(),
        location: value_label(),
        session: value_label(),
        time_on_server: value_label(),
    };

    let grid = gtk::Grid::builder()
        .row_spacing(8)
        .column_spacing(16)
        .build();
    let rows = [
        ("Game", &fields.game),
        ("Place ID", &fields.place),
        ("Server ID", &fields.server),
        ("Address", &fields.address),
        ("Location", &fields.location),
        ("Session", &fields.session),
        ("In this server", &fields.time_on_server),
    ];
    for (row, (name, value)) in rows.into_iter().enumerate() {
        let name = gtk::Label::builder()
            .label(name)
            .xalign(0.0)
            .valign(gtk::Align::Start)
            .css_classes(["dim-label"])
            .build();
        grid.attach(&name, 0, row as i32, 1, 1);
        grid.attach(value, 1, row as i32, 1, 1);
    }

    let copy_link = gtk::Button::with_label("Copy join link");
    copy_on_click(&copy_link, {
        let state = state.clone();
        move || {
            CurrentServer::of(&state)
                .map(|server| backend::game_link(server.game.place_id, &server.server.job_id))
                .unwrap_or_default()
        }
    });
    let copy_server_id = gtk::Button::with_label("Copy server ID");
    copy_on_click(&copy_server_id, {
        let state = state.clone();
        move || {
            CurrentServer::of(&state)
                .map(|server| server.server.job_id)
                .unwrap_or_default()
        }
    });
    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    actions.set_halign(gtk::Align::End);
    actions.append(&copy_link);
    actions.append(&copy_server_id);

    let body = gtk::Box::new(gtk::Orientation::Vertical, 12);
    body.add_css_class("overlay-body");
    body.append(&grid);
    body.append(&gtk::Box::builder().vexpand(true).build());
    body.append(&actions);

    show(state, &fields);
    let fields = std::rc::Rc::new(fields);
    for property in PROPERTIES_THAT_DECIDE_CONTENT {
        state.connect_notify_local(
            Some(property),
            glib::clone!(
                #[strong]
                fields,
                move |state, _| show(state, &fields)
            ),
        );
    }
    every_second_while_shown(&body, {
        let state = state.clone();
        move |_| show(&state, &fields)
    });

    body.upcast()
}

fn value_label() -> gtk::Label {
    gtk::Label::builder()
        .xalign(0.0)
        .hexpand(true)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .build()
}

fn show(state: &AppState, fields: &Fields) {
    fields.session.set_label(&session_time(state));

    let Some(current) = CurrentServer::of(state) else {
        for label in [
            &fields.game,
            &fields.place,
            &fields.server,
            &fields.address,
            &fields.location,
            &fields.time_on_server,
        ] {
            label.set_label(NOT_AVAILABLE);
        }
        return;
    };

    fields.game.set_label(&current.game_name());
    fields.place.set_label(&current.game.place_id.to_string());
    fields.server.set_label(&current.server.job_id);
    fields
        .address
        .set_label(current.server.address.as_deref().unwrap_or(NOT_AVAILABLE));
    fields
        .location
        .set_label(current.server.location.as_deref().unwrap_or(NOT_AVAILABLE));
    fields.time_on_server.set_label(&current.time_on_server());
}
