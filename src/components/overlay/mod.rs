mod shortcut;

pub use shortcut::listen_for_toggle_overlay_shortcut;

use adw::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

const OVERLAY_BACKDROP_STYLE: &str = "overlay-backdrop";
const EDGES_THAT_FILL_THE_SCREEN: [Edge; 4] = [Edge::Top, Edge::Bottom, Edge::Left, Edge::Right];
const NAMESPACE_COMPOSITORS_TREAT_AS_ON_SCREEN_DISPLAY: &str = "on-screen-display";
const FADE_IN_MILLISECONDS: u32 = 150;

pub fn build_overlay(app: &adw::Application) -> gtk::Window {
    let overlay = gtk::Window::builder()
        .application(app)
        .css_classes([OVERLAY_BACKDROP_STYLE])
        .build();

    overlay.init_layer_shell();
    overlay.set_namespace(Some(NAMESPACE_COMPOSITORS_TREAT_AS_ON_SCREEN_DISPLAY));
    overlay.set_layer(Layer::Overlay);
    overlay.set_keyboard_mode(KeyboardMode::OnDemand);
    for edge in EDGES_THAT_FILL_THE_SCREEN {
        overlay.set_anchor(edge, true);
    }
    overlay.set_exclusive_zone(-1);
    fade_in_whenever_shown(&overlay);

    overlay
}

fn fade_in_whenever_shown(overlay: &gtk::Window) {
    let opacity = adw::PropertyAnimationTarget::new(overlay, "opacity");
    let fade_in = adw::TimedAnimation::new(overlay, 0.0, 1.0, FADE_IN_MILLISECONDS, opacity);
    fade_in.set_easing(adw::Easing::Linear);
    overlay.connect_map(move |_| fade_in.play());
}
