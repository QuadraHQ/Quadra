fn lightning() -> f32 {
    let time = scene.seconds;
    let storm = pow(max(sin(time * 0.37) * sin(time * 0.23 + 1.3), 0.0), 18.0);
    let flicker = 0.6 + 0.4 * sin(time * 47.0);
    return storm * flicker;
}

fn glow(spot: vec2<f32>) -> vec3<f32> {
    let down = spot.y / scene.size.y;
    let mist = smoothstep(0.4, 1.0, down) * 0.05;
    let flash = lightning() * (1.0 - down * 0.6) * 0.3;
    return scene.base.rgb + tint(0u) * mist + tint(1u) * flash;
}

fn light_at(covering: Covering) -> vec3<f32> {
    return sampled_glow(covering);
}

@vertex
fn place_mote(@builtin(vertex_index) corner: u32, @builtin(instance_index) drop: u32) -> Mote {
    let depth = random(drop, 1u);
    let fall = mix(520.0, 1000.0, depth);
    let span = scene.size.y + 120.0;
    let travelled = scene.seconds * fall + random(drop, 2u) * span;
    let round = u32(travelled / span) * 8u;
    let slant = vec2<f32>(0.16, 1.0);
    let heading = normalize(slant);
    let start_x = random(drop, 3u + round) * (scene.size.x + 160.0) - 80.0;
    let down = travelled - floor(travelled / span) * span - 60.0;
    let center = vec2<f32>(start_x + down * slant.x, down);
    var mote = streak_at(center, mix(0.5, 0.9, depth), heading, mix(7.0, 13.0, depth), corner);
    mote.tint = tint(0u);
    mote.strength = mix(0.12, 0.34, depth) * (1.0 + lightning());
    return mote;
}
