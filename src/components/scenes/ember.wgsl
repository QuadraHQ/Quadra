const WINDOW_LIGHT = vec3<f32>(0.008, 0.0056, 0.0044);
const SOOT = vec3<f32>(0.086, 0.067, 0.055);
const SMOULDER = vec3<f32>(0.17, 0.088, 0.047);
const FLAME = vec3<f32>(1.0, 0.4, 0.11);
const WHITE_HOT = vec3<f32>(1.0, 0.82, 0.5);

fn heat(spot: vec2<f32>) -> f32 {
    let time = scene.seconds;
    let flicker = noise(vec2<f32>(spot.x / 150.0 + time * 0.12, time * 0.45)) * 0.6
        + noise(vec2<f32>(spot.x / 55.0 - time * 0.2, time * 0.9 + 9.0)) * 0.4;
    let rise = smoothstep(0.5, 1.08, spot.y / scene.size.y + flicker * 0.2);
    return rise * rise;
}

fn glow(spot: vec2<f32>) -> vec3<f32> {
    let base = mix(SOOT, SMOULDER, smoothstep(0.25, 1.0, spot.y / scene.size.y));
    return base + FLAME * heat(spot) * 0.2;
}

fn light_at(covering: Covering) -> vec3<f32> {
    return sampled_glow(covering);
}

@vertex
fn place_mote(@builtin(vertex_index) corner: u32, @builtin(instance_index) ember: u32) -> Mote {
    let time = scene.seconds;
    let lifetime = mix(7.0, 14.0, random(ember, 1u));
    let age = time / lifetime + random(ember, 2u);
    let life = u32(age) * 16u;
    let progress = fract(age);
    let start = random(ember, 3u + life) * scene.size.x;
    let climb = mix(0.3, 0.85, random(ember, 4u + life)) * scene.size.y;
    let sway = sin(time * mix(0.5, 1.3, random(ember, 5u)) + random(ember, 6u) * TAU)
        * mix(5.0, 18.0, random(ember, 7u)) * progress;
    let drift = (random(ember, 8u + life) - 0.35) * 70.0 * progress;
    let center = vec2<f32>(start + sway + drift, scene.size.y + 8.0 - progress * climb);
    let fading = smoothstep(0.0, 0.08, progress) * pow(1.0 - progress, 1.4);
    let flicker = 0.7 + 0.3 * sin(time * mix(7.0, 13.0, random(ember, 10u)) + random(ember, 11u) * TAU);

    var mote = mote_at(center, mix(1.1, 2.3, random(ember, 9u)), corner);
    mote.tint = mix(FLAME, WHITE_HOT, pow(1.0 - progress, 2.0));
    mote.strength = fading * flicker * mix(0.55, 1.0, random(ember, 12u));
    return mote;
}
