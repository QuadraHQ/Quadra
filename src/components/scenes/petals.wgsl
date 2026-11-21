fn glow(spot: vec2<f32>) -> vec3<f32> {
    let uv = spot / scene.size;
    let blush = (1.0 - uv.x) * (1.0 - uv.y) * 0.08 + uv.x * uv.y * 0.04;
    return mix(scene.base.rgb, tint(0u), blush);
}

fn light_at(covering: Covering) -> vec3<f32> {
    return sampled_glow(covering);
}

@vertex
fn place_mote(@builtin(vertex_index) corner: u32, @builtin(instance_index) petal: u32) -> Mote {
    let time = scene.seconds;
    let life = mix(9.0, 16.0, random(petal, 1u));
    let age = time / life + random(petal, 2u);
    let round = u32(age) * 8u;
    let progress = fract(age);
    let start = vec2<f32>(random(petal, 3u + round) * (scene.size.x + 200.0) - 260.0, -30.0);
    let flight = vec2<f32>(mix(140.0, 320.0, random(petal, 4u + round)), scene.size.y + 60.0);
    let flutter = vec2<f32>(sin(time * 1.3 + random(petal, 5u) * TAU) * 18.0, cos(time * 0.9 + random(petal, 6u) * TAU) * 8.0);
    let spin = time * mix(0.6, 1.8, random(petal, 7u)) + random(petal, 8u) * TAU;
    let tumble = abs(sin(time * mix(0.8, 1.6, random(petal, 9u)) + random(petal, 10u) * TAU));
    let heading = vec2<f32>(cos(spin), sin(spin));
    let center = start + flight * progress + flutter;
    var mote = streak_at(center, mix(2.2, 3.6, random(petal, 11u)) * (0.45 + 0.55 * tumble), heading, 1.7, corner);
    mote.tint = mix(tint(1u), tint(2u), random(petal, 12u));
    mote.strength = smoothstep(0.0, 0.05, progress) * (1.0 - smoothstep(0.9, 1.0, progress)) * mix(0.35, 0.6, tumble);
    return mote;
}
