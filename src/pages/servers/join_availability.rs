use crate::components::play_action::{PROPERTIES_THAT_DECIDE_PLAY_ACTION, PlayAction};
use crate::state::{AppState, Client};
use adw::prelude::*;
use gtk::glib;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

#[derive(Clone)]
pub struct JoinAvailability {
    state: AppState,
    join_widgets: Rc<RefCell<Vec<glib::WeakRef<gtk::Widget>>>>,
}

impl JoinAvailability {
    pub fn follow(state: &AppState) -> Self {
        let availability = Self {
            state: state.clone(),
            join_widgets: Rc::default(),
        };

        let followed_widgets: Weak<_> = Rc::downgrade(&availability.join_widgets);
        for property in PROPERTIES_THAT_DECIDE_PLAY_ACTION {
            let followed_widgets = followed_widgets.clone();
            state.connect_notify_local(Some(property), move |state, _| {
                if let Some(join_widgets) = followed_widgets.upgrade() {
                    show_availability(state, &join_widgets.borrow());
                }
            });
        }

        availability
    }

    pub fn applies_to(&self, widget: &impl IsA<gtk::Widget>) {
        widget.set_sensitive(can_join(&self.state));
        let mut join_widgets = self.join_widgets.borrow_mut();
        join_widgets.retain(|widget| widget.upgrade().is_some());
        join_widgets.push(widget.upcast_ref::<gtk::Widget>().downgrade());
    }
}

fn show_availability(state: &AppState, join_widgets: &[glib::WeakRef<gtk::Widget>]) {
    let can_join = can_join(state);
    for widget in join_widgets.iter().filter_map(glib::WeakRef::upgrade) {
        widget.set_sensitive(can_join);
    }
}

fn can_join(state: &AppState) -> bool {
    let action = PlayAction::for_client(state, Client::Player);
    action == PlayAction::Play(Client::Player) && action.is_available(state)
}
