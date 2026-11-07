mod history_group;
mod join_availability;
mod join_by_link_row;
mod place_link;
mod region_row;
mod time_since;

use crate::backend;
use crate::state::AppState;
use crate::utils::{scrollable, unfocus_on_outside_click};
use adw::prelude::*;
use gtk::glib;
use history_group::history_groups;
use join_availability::JoinAvailability;
use join_by_link_row::join_by_link_row;
use region_row::preferred_region_row;

pub fn servers_page(state: &AppState) -> gtk::Widget {
    let join_availability = JoinAvailability::follow(state);

    let joining_group = adw::PreferencesGroup::builder()
        .title("Joining")
        .description(
            "With a preferred region, games started from this page join a server there, \
             picked from RoValra's server list (apis.rovalra.com). RoValra sees the game and \
             the region, not your account.",
        )
        .build();
    joining_group.add(&preferred_region_row(state));
    joining_group.add(&join_by_link_row(state, &join_availability));

    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .margin_top(12)
        .margin_bottom(12)
        .build();
    content.append(&joining_group);
    for group in history_groups(state, &join_availability) {
        content.append(&group);
    }

    let page = scrollable(&content);
    unfocus_on_outside_click(&page);
    load_regions_when_first_shown(&page, state);

    page.upcast()
}

fn load_regions_when_first_shown(page: &gtk::ScrolledWindow, state: &AppState) {
    let handler = std::rc::Rc::new(std::cell::Cell::new(None));
    let first_map = page.connect_map(glib::clone!(
        #[strong]
        state,
        #[strong]
        handler,
        move |page| {
            backend::load_regions(&state);
            if let Some(handler) = handler.take() {
                page.disconnect(handler);
            }
        }
    ));
    handler.set(Some(first_map));
}
