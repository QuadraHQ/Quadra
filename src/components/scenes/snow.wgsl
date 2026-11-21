fn glow(spot: vec2<f32>) -> vec3<f32> {
    let down = spot.y / scene.size.y;
    let drift = smoothstep(0.75, 1.0, down) * 0.07;
    let sky = mix(scene.base.rgb, tint(1u), 0.05 * (1.0 - down));
    return sky + tint(1u) * drift;
}

fn light_at(covering: Covering) -> vec3<f32> {
    return sampled_glow(covering);
}

@vertex
fn place_mote(@builtin(vertex_index) corner: u32, @builtin(instance_index) flake: u32) -> Mote {
    let time = scene.seconds;
    let depth = pow(random(flake, 1u), 1.8);
    let fall = mix(14.0, 55.0, depth);
    let span = scene.size.y + 40.0;
    let travelled = time * fall + random(flake, 2u) * span;
    let down = travelled - floor(travelled / span) * span - 20.0;
    let sway = sin(time * mix(0.3, 0.8, random(flake, 3u)) + random(flake, 4u) * TAU) * mix(8.0, 30.0, depth);
    let wind = time * mix(3.0, 12.0, depth);
    let across = fract((random(flake, 5u) * scene.size.x + wind + sway) / (scene.size.x + 40.0)) * (scene.size.x + 40.0) - 20.0;
    var mote = mote_at(vec2<f32>(across, down), mix(0.7, 2.2, depth), corner);
    mote.tint = mix(tint(0u), tint(1u), random(flake, 6u) * 0.5);
    mote.strength = mix(0.25, 0.85, depth);
    return mote;
}
