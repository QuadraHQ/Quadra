use gtk::prelude::*;

pub fn scrollable(child: &impl IsA<gtk::Widget>) -> gtk::ScrolledWindow {
    let clamp = adw::Clamp::builder().maximum_size(600).child(child).build();

    gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&clamp)
        .vexpand(true)
        .build()
}

pub fn unfocus_on_outside_click(container: &impl IsA<gtk::Widget>) {
    let gesture = gtk::GestureClick::new();
    // Capture phase, so this runs before the clicked child handles the event.
    gesture.set_propagation_phase(gtk::PropagationPhase::Capture);

    gesture.connect_pressed(|gesture, _, x, y| {
        let Some(container) = gesture.widget() else {
            return;
        };
        let Some(root) = container.root() else { return };
        let Some(focus) = root.focus() else { return };

        // In an Entry, the focus sits on its inner gtk::Text child.
        if !focus.is::<gtk::Text>() {
            return;
        }
        let field = focus.parent().unwrap_or(focus);

        let clicked = container.pick(x, y, gtk::PickFlags::DEFAULT);
        let inside = clicked.is_some_and(|w| w == field || w.is_ancestor(&field));
        if !inside {
            root.set_focus(None::<&gtk::Widget>);
        }
    });

    container.add_controller(gesture);
}
