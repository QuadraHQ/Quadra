fn glow(spot: vec2<f32>) -> vec3<f32> {
    let uv = spot / scene.size;
    let centered = uv - vec2<f32>(0.5, 0.65);
    let dusk = smoothstep(0.35, 1.0, uv.y) * 0.05;
    let vignette = 1.0 - dot(centered, centered) * 0.35;
    return (scene.base.rgb + tint(0u) * dusk) * vignette;
}

fn light_at(covering: Covering) -> vec3<f32> {
    return sampled_glow(covering);
}

@vertex
fn place_mote(@builtin(vertex_index) corner: u32, @builtin(instance_index) fly: u32) -> Mote {
    let time = scene.seconds;
    let home = vec2<f32>(random(fly, 1u), mix(0.25, 0.95, random(fly, 2u))) * scene.size;
    let roam = vec2<f32>(
        sin(time * mix(0.08, 0.2, random(fly, 3u)) + random(fly, 4u) * TAU) * 70.0
            + sin(time * mix(0.3, 0.6, random(fly, 5u)) + random(fly, 6u) * TAU) * 18.0,
        cos(time * mix(0.07, 0.18, random(fly, 7u)) + random(fly, 8u) * TAU) * 45.0
            + sin(time * mix(0.4, 0.7, random(fly, 9u))) * 10.0,
    );
    let pulse = 0.5 + 0.5 * sin(time * mix(0.6, 1.3, random(fly, 10u)) + random(fly, 11u) * TAU);
    let blink = pow(smoothstep(0.35, 1.0, pulse), 2.0);
    var mote = mote_at(home + roam, mix(1.5, 2.6, random(fly, 12u)), corner);
    mote.tint = mix(tint(0u), tint(1u), step(0.7, random(fly, 13u)));
    mote.strength = 0.08 + blink * mix(0.7, 1.0, random(fly, 14u));
    return mote;
}
