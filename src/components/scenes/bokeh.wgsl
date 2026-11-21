fn glow(spot: vec2<f32>) -> vec3<f32> {
    let uv = spot / scene.size;
    let haze = tint(0u) * (1.0 - uv.x) * uv.y * 0.05 + tint(1u) * uv.x * (1.0 - uv.y) * 0.05;
    return scene.base.rgb + haze;
}

fn light_at(covering: Covering) -> vec3<f32> {
    return sampled_glow(covering);
}

@vertex
fn place_mote(@builtin(vertex_index) corner: u32, @builtin(instance_index) light: u32) -> Mote {
    let time = scene.seconds;
    let radius = mix(14.0, 46.0, pow(random(light, 1u), 1.6));
    let span = scene.size + radius * 4.0;
    let drift = vec2<f32>(mix(-6.0, 6.0, random(light, 2u)), -mix(4.0, 11.0, random(light, 3u)));
    let start = vec2<f32>(random(light, 4u), random(light, 5u)) * span;
    let wander = vec2<f32>(sin(time * 0.11 + random(light, 6u) * TAU), cos(time * 0.09 + random(light, 7u) * TAU)) * 12.0;
    let travelled = start + drift * time;
    let place = travelled - floor(travelled / span) * span - radius * 2.0 + wander;
    let breathing = 0.75 + 0.25 * sin(time * mix(0.2, 0.5, random(light, 8u)) + random(light, 9u) * TAU);
    var mote = disc_at(place, radius, corner);
    mote.tint = tint(u32(random(light, 10u) * 2.999));
    mote.strength = mix(0.06, 0.16, random(light, 11u)) * breathing;
    return mote;
}
