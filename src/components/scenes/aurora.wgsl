fn sky(uv: vec2<f32>) -> vec3<f32> {
    let along = clamp((uv.x + uv.y) * 0.5, 0.0, 1.0);
    let upper = mix(mix(scene.base.rgb, tint(1u), 0.18), scene.base.rgb, smoothstep(0.0, 0.5, along));
    return mix(upper, mix(scene.base.rgb, tint(0u), 0.14), smoothstep(0.5, 1.0, along));
}

fn curtain(spot: vec2<f32>, edge: f32, speed: f32, phase: f32) -> vec3<f32> {
    let across = spot.x / scene.size.y;
    let down = spot.y / scene.size.y;
    let time = scene.seconds * speed;
    let hem = edge
        + 0.06 * sin(across * 1.6 + time + phase)
        + 0.03 * sin(across * 3.7 - time * 1.7 + phase * 2.3)
        + 0.012 * sin(across * 8.9 + time * 2.9 + phase);
    let height = hem - down;
    let rising = exp(-max(height, 0.0) / 0.13);
    let below = min(height, 0.0) / 0.02;
    let hanging = rising * exp(-below * below);
    let rays = mix(0.4, 1.0, noise(vec2<f32>(across * 24.0 + phase * 7.0, time * 1.4 + phase)));
    let shimmer = 0.75 + 0.25 * sin(scene.seconds * 0.23 + phase * 3.1);
    return mix(tint(0u), tint(1u), smoothstep(0.0, 0.2, height)) * hanging * rays * shimmer;
}

fn glow(spot: vec2<f32>) -> vec3<f32> {
    let lights = curtain(spot, 0.2, 0.11, 0.0) * 0.34
        + curtain(spot, 0.32, 0.08, 2.1) * 0.24
        + curtain(spot, 0.43, 0.06, 4.4) * 0.16;
    return sky(spot / scene.size) + lights;
}

fn light_at(covering: Covering) -> vec3<f32> {
    return sampled_glow(covering);
}

@vertex
fn place_mote(@builtin(vertex_index) corner: u32, @builtin(instance_index) star: u32) -> Mote {
    let place = vec2<f32>(random(star, 1u), random(star, 2u) * random(star, 3u) * 0.8) * scene.size;
    let twinkle = 0.5 + 0.5 * sin(scene.seconds * mix(0.6, 2.2, random(star, 4u)) + random(star, 5u) * TAU);
    var mote = mote_at(place, mix(0.5, 0.95, random(star, 6u)), corner);
    mote.tint = tint(2u);
    mote.strength = mix(0.2, 0.7, random(star, 7u)) * twinkle;
    return mote;
}
