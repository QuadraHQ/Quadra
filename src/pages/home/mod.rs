mod client_page;
mod logs_button;
mod play_button;
mod status_label;

use crate::state::{AppState, Client};
use adw::prelude::*;
use client_page::client_page;
use gtk::glib;

pub fn home_page(state: &AppState) -> gtk::Widget {
    let clients = adw::ViewStack::builder().vexpand(true).build();
    for client in Client::ALL {
        clients.add_titled(
            &client_page(state, client),
            Some(client.page_name()),
            client.short_name(),
        );
    }
    clients.set_visible_child_name(state.selected_client().page_name());
    clients.connect_visible_child_name_notify(glib::clone!(
        #[weak]
        state,
        move |clients| {
            let shown_client = clients
                .visible_child_name()
                .and_then(|name| Client::with_page_name(&name));
            if let Some(client) = shown_client {
                state.set_selected_client(client);
            }
        }
    ));

    let switcher = adw::InlineViewSwitcher::builder()
        .stack(&clients)
        .halign(gtk::Align::Center)
        .margin_top(12)
        .build();

    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    content.append(&switcher);
    content.append(&clients);

    content.upcast()
}
