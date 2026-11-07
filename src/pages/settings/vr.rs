use super::bound_rows::switch_row;
use crate::state::{AppState, VR_RENDER_SCALE_RANGE};
use adw::prelude::*;

const RENDER_SCALE_STEP: f64 = 5.0;

pub fn vr_section(state: &AppState) -> gtk::Box {
    let vr_row = switch_row(state, "vr", "Play in VR");
    state
        .bind_property("vr-status", &vr_row, "subtitle")
        .sync_create()
        .build();

    let mirror_row = switch_row(state, "vr-mirror", "Show the game in its window too");
    mirror_row.set_subtitle("Mirrors what the headset shows into Roblox's window");

    let zink_row = switch_row(state, "vr-zink", "OpenGL on Vulkan (zink)");
    zink_row.set_subtitle("Runs Roblox's OpenGL renderer on Mesa's Vulkan driver");

    let group = adw::PreferencesGroup::builder()
        .title("VR")
        .description(
            "Plays Roblox on a headset through this computer's OpenXR runtime, \
             like WiVRn or Monado. VR uses Roblox's OpenGL renderer.",
        )
        .build();
    group.add(&vr_row);
    for row in [
        mirror_row.upcast::<gtk::Widget>(),
        render_scale_row(state).upcast(),
        zink_row.upcast(),
    ] {
        state
            .bind_property("vr", &row, "sensitive")
            .sync_create()
            .build();
        group.add(&row);
    }

    let section = gtk::Box::new(gtk::Orientation::Vertical, 12);
    section.append(&group);

    section
}

fn render_scale_row(state: &AppState) -> adw::SpinRow {
    let row = adw::SpinRow::with_range(
        f64::from(*VR_RENDER_SCALE_RANGE.start()),
        f64::from(*VR_RENDER_SCALE_RANGE.end()),
        RENDER_SCALE_STEP,
    );
    row.set_title("Render scale (%)");
    row.set_subtitle("Percent of the headset's recommended resolution");

    state
        .bind_property("vr-render-scale", &row, "value")
        .transform_to(|_, scale: u32| Some(f64::from(scale)))
        .transform_from(|_, value: f64| Some(value.round() as u32))
        .bidirectional()
        .sync_create()
        .build();

    row
}
