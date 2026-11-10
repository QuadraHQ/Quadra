use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

const OVERLAY_BACKDROP_STYLE: &str = "overlay-backdrop";
const EDGES_THAT_FILL_THE_SCREEN: [Edge; 4] = [Edge::Top, Edge::Bottom, Edge::Left, Edge::Right];

pub fn build_overlay(app: &adw::Application) -> gtk::Window {
    let overlay = gtk::Window::builder()
        .application(app)
        .css_classes([OVERLAY_BACKDROP_STYLE])
        .build();

    overlay.init_layer_shell();
    overlay.set_layer(Layer::Overlay);
    overlay.set_keyboard_mode(KeyboardMode::OnDemand);
    for edge in EDGES_THAT_FILL_THE_SCREEN {
        overlay.set_anchor(edge, true);
    }

    overlay
}
