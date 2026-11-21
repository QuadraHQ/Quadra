const SCANLINE_SPACING = 3.0;
const SCANLINE_LIGHT = 0.035;
const BAND_LIGHT = 0.0045;
const ROLL_SECONDS = 8.0;

fn rolling_band(down: f32) -> f32 {
    let roll = fract(scene.seconds / ROLL_SECONDS) * 1.5 - 0.25;
    let distance = (down - roll) / 0.07;
    return exp(-distance * distance);
}

fn light_at(covering: Covering) -> vec3<f32> {
    let spot = spot_of(covering);
    let centered = covering.uv - 0.5;
    let vignette = 1.0 - dot(centered, centered) * 1.2;
    let band = rolling_band(covering.uv.y);
    let row = spot.y - SCANLINE_SPACING * floor(spot.y / SCANLINE_SPACING);
    let scanline = select(0.0, SCANLINE_LIGHT, row < 1.0) * (1.0 + band * 0.6);
    let breathing = 0.0006 * sin(scene.seconds * 0.4);
    let phosphor = scene.tint_lights[0].rgb;
    return scene.base_light.rgb * vignette + phosphor * ((band * BAND_LIGHT + breathing) * vignette + scanline);
}
