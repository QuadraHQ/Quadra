use gtk::prelude::*;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

pub fn scrollable(child: &impl IsA<gtk::Widget>) -> gtk::ScrolledWindow {
    let clamp = adw::Clamp::builder().maximum_size(600).child(child).build();

    gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&clamp)
        .vexpand(true)
        .build()
}

pub fn scroll_into_view_once_shown(widget: &impl IsA<gtk::Widget>) {
    widget.add_tick_callback(|widget, _| {
        let is_laid_out = widget.width() > 0;
        if !is_laid_out {
            return gtk::glib::ControlFlow::Continue;
        }

        let viewport = widget
            .ancestor(gtk::Viewport::static_type())
            .and_downcast::<gtk::Viewport>();
        if let Some(viewport) = viewport {
            viewport.scroll_to(widget, None);
        }
        gtk::glib::ControlFlow::Break
    });
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

pub fn find_in_path(program: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|directory| directory.join(program))
        .find(|candidate| is_executable_file(candidate))
}

pub fn is_executable_file(path: &std::path::Path) -> bool {
    path.metadata()
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
}

pub fn path_for_display(path: &std::path::Path) -> String {
    match path.strip_prefix(gtk::glib::home_dir()) {
        Ok(path_inside_home) => format!("~/{}", path_inside_home.display()),
        Err(_) => path.display().to_string(),
    }
}
