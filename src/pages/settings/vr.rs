use super::bound_rows::{SpinRange, spin_row, switch_row, switch_row_when};
use crate::components::preferences::preference_group;
use crate::components::style::Look;
use crate::quadra::Message;
use crate::state::{AppState, VR_RENDER_SCALE_RANGE};
use iced::Element;

const RENDER_SCALE_STEP: u32 = 5;

pub fn vr_section<'a>(state: &AppState, look: Look) -> Vec<Element<'a, Message>> {
    let vr_enabled = state.vr();

    let group = preference_group(look)
        .title("VR")
        .description(
            "Plays Roblox on a headset through this computer's OpenXR runtime, \
             like WiVRn or Monado. VR uses Roblox's OpenGL renderer.",
        )
        .push(
            switch_row(look, "Play in VR", vr_enabled, |state, on| state.set_vr(on))
                .subtitle(state.vr_status()),
        )
        .push(
            switch_row_when(
                look,
                "Show the game in its window too",
                state.vr_mirror(),
                |state, on| state.set_vr_mirror(on),
                vr_enabled,
            )
            .subtitle("Mirrors what the headset shows into Roblox's window"),
        )
        .push(
            spin_row(
                look,
                "Render scale (%)",
                state.vr_render_scale(),
                SpinRange {
                    values: VR_RENDER_SCALE_RANGE,
                    step: RENDER_SCALE_STEP,
                },
                |state, scale| state.set_vr_render_scale(scale),
                vr_enabled,
            )
            .subtitle("Percent of the headset's recommended resolution"),
        )
        .push(
            switch_row_when(
                look,
                "OpenGL on Vulkan (zink)",
                state.vr_zink(),
                |state, on| state.set_vr_zink(on),
                vr_enabled,
            )
            .subtitle("Runs Roblox's OpenGL renderer on Mesa's Vulkan driver"),
        );

    vec![group.into()]
}
