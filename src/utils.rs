use crate::state::{AppState, Client};
use gtk::prelude::*;
use serde::de::DeserializeOwned;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

const USER_AGENT: &str = "Quadra";

pub fn scrollable(child: &impl IsA<gtk::Widget>) -> gtk::ScrolledWindow {
    let clamp = adw::Clamp::builder().maximum_size(600).child(child).build();

    gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&clamp)
        .vexpand(true)
        .build()
}

pub const APP_ICON_PNG: &[u8] = include_bytes!("../data/icons/org.quadrahq.Quadra.png");

pub fn show_only_while_roblox_is_selected(page: &adw::ViewStackPage, state: &AppState) {
    state
        .bind_property("selected-client", page, "visible")
        .transform_to(|_, client: Client| Some(client == Client::Player))
        .sync_create()
        .build();
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

pub fn is_running_in_flatpak() -> bool {
    std::path::Path::new("/.flatpak-info").exists()
}

pub fn files_under(path: &std::path::Path) -> std::io::Result<Vec<PathBuf>> {
    if !path.is_dir() {
        return Ok(vec![path.to_owned()]);
    }
    let mut files = Vec::new();
    for entry in std::fs::read_dir(path)? {
        files.extend(files_under(&entry?.path())?);
    }
    files.sort();
    Ok(files)
}

pub fn create_parent_directory(path: &std::path::Path) -> std::io::Result<()> {
    match path.parent() {
        Some(parent) => std::fs::create_dir_all(parent),
        None => Ok(()),
    }
}

pub fn decode_base64(text: &str) -> Option<Vec<u8>> {
    let mut decoded = Vec::with_capacity(text.len() * 3 / 4);
    let mut buffer = 0u32;
    let mut bits = 0;
    for byte in text.trim_end_matches('=').bytes() {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            _ => return None,
        };
        buffer = buffer << 6 | u32::from(value);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            decoded.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }
    Some(decoded)
}

pub fn get_json<T: DeserializeOwned>(url: &str) -> anyhow::Result<T> {
    ureq::get(url)
        .header("User-Agent", USER_AGENT)
        .call()
        .and_then(|mut response| response.body_mut().read_json())
        .with_context(|| format!("Couldn't get {url}"))
}

#[cfg(test)]
mod tests {
    use super::decode_base64;

    #[test]
    fn decodes_with_and_without_padding() {
        assert_eq!(decode_base64("aGVsbG8=").unwrap(), b"hello");
        assert_eq!(decode_base64("aGVsbG8").unwrap(), b"hello");
        assert_eq!(decode_base64("eyJBIjoxfQ").unwrap(), br#"{"A":1}"#);
        assert!(decode_base64("a*b").is_none());
    }
}

pub fn format_playtime(seconds: u64) -> String {
    let minutes = seconds / 60;
    match (minutes / 60, minutes % 60) {
        (0, minutes) => format!("{minutes} min"),
        (hours, 0) => format!("{hours} h"),
        (hours, minutes) => format!("{hours} h {minutes} min"),
    }
}

#[cfg(test)]
mod playtime_tests {
    use super::format_playtime;

    #[test]
    fn formats_playtime_compactly() {
        assert_eq!(format_playtime(0), "0 min");
        assert_eq!(format_playtime(59), "0 min");
        assert_eq!(format_playtime(60 * 45), "45 min");
        assert_eq!(format_playtime(3600 * 2), "2 h");
        assert_eq!(format_playtime(3600 * 2 + 60 * 15), "2 h 15 min");
    }
}
