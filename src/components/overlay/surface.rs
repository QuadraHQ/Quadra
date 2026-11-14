use gtk::gdk;
use gtk::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

const STYLE: &str = include_str!("style.css");

pub fn is_supported() -> bool {
    gtk4_layer_shell::is_supported()
}

pub fn load_style() {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(STYLE);
    gtk::style_context_add_provider_for_display(
        &gdk::Display::default().expect("a display is open once the app has started"),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

pub fn window() -> gtk::Window {
    let window = gtk::Window::builder()
        .title("Quadra overlay")
        .decorated(false)
        .focusable(false)
        .css_classes(["overlay-surface"])
        .build();

    window.init_layer_shell();
    window.set_namespace(Some("quadra-overlay"));
    window.set_layer(Layer::Overlay);
    window.set_exclusive_zone(-1);
    window.set_keyboard_mode(KeyboardMode::None);
    for edge in [Edge::Top, Edge::Bottom, Edge::Left, Edge::Right] {
        window.set_anchor(edge, true);
    }

    window
}

pub fn accept_keyboard(window: &gtk::Window, accepts_keyboard: bool) {
    window.set_keyboard_mode(if accepts_keyboard {
        KeyboardMode::OnDemand
    } else {
        KeyboardMode::None
    });
}

pub fn accept_input_only_inside(window: &gtk::Window, areas: &[gdk::Rectangle]) {
    let Some(surface) = window.surface() else {
        return;
    };
    let rectangles: Vec<gtk::cairo::RectangleInt> = areas
        .iter()
        .map(|area| gtk::cairo::RectangleInt::new(area.x(), area.y(), area.width(), area.height()))
        .collect();
    surface.set_input_region(Some(&gtk::cairo::Region::create_rectangles(&rectangles)));
}

pub fn area_of(widget: &impl IsA<gtk::Widget>, window: &gtk::Window) -> Option<gdk::Rectangle> {
    let bounds = widget.compute_bounds(window)?;
    Some(gdk::Rectangle::new(
        bounds.x().floor() as i32,
        bounds.y().floor() as i32,
        bounds.width().ceil() as i32,
        bounds.height().ceil() as i32,
    ))
}
