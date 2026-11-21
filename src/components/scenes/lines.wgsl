const LINE_SPACING = 300.0;
const CONTOURS = 9.0;

fn brightness_of(color: vec3<f32>) -> f32 {
    return dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
}

fn flowing(place: vec2<f32>, seconds: f32) -> f32 {
    let drift = vec2<f32>(
        noise(place * 0.7 + vec2<f32>(seconds * 0.021, 3.1)),
        noise(place * 0.7 + vec2<f32>(7.4, seconds * 0.017)),
    );
    let warped = place + drift * 1.7;
    return noise(warped + vec2<f32>(0.0, seconds * 0.012)) * 0.7
        + noise(warped * 2.3 - drift + vec2<f32>(seconds * 0.009, 0.0)) * 0.3;
}

fn light_at(covering: Covering) -> vec3<f32> {
    let spot = spot_of(covering);
    let place = spot / LINE_SPACING;
    let height = flowing(place, scene.seconds) * CONTOURS;
    let width = max(fwidth(height), 0.0001);
    let gap = min(fract(height), 1.0 - fract(height));
    let line = 1.0 - smoothstep(width * 0.45, width * 1.35, gap);
    let shine = smoothstep(0.55, 0.85, noise(place * 0.9 + vec2<f32>(scene.seconds * 0.05, -scene.seconds * 0.03)));
    let base = scene.base_light.rgb;
    let lines = scene.tint_lights[0].rgb;
    let highlight = scene.tint_lights[1].rgb;
    if brightness_of(base) > 0.4 {
        let ink = base * mix(vec3<f32>(0.55), lines / max(brightness_of(lines), 0.05) * 0.55, 0.3);
        return mix(base, ink, line * (0.2 + 0.3 * shine));
    }
    let color = mix(lines, highlight, shine);
    return mix(base, color, line * (0.09 + 0.24 * shine));
}
