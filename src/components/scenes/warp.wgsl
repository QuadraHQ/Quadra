fn glow(spot: vec2<f32>) -> vec3<f32> {
    let centered = (spot - scene.size * 0.5) / scene.size.y;
    let pull = exp(-dot(centered, centered) * 9.0);
    let swirl = 0.5 + 0.5 * sin(atan2(centered.y, centered.x) * 3.0 + scene.seconds * 0.4);
    return scene.base.rgb + tint(0u) * pull * (0.1 + 0.05 * swirl);
}

fn light_at(covering: Covering) -> vec3<f32> {
    return sampled_glow(covering);
}

@vertex
fn place_mote(@builtin(vertex_index) corner: u32, @builtin(instance_index) star: u32) -> Mote {
    let age = scene.seconds * mix(0.12, 0.22, random(star, 1u)) + random(star, 2u);
    let journey = u32(age) * 8u;
    let progress = fract(age);
    let angle = random(star, 3u + journey) * TAU;
    let heading = vec2<f32>(cos(angle), sin(angle));
    let reach = length(scene.size) * 0.55;
    let start = mix(0.02, 0.12, random(star, 4u + journey)) * reach;
    let center = scene.size * 0.5 + heading * (start + pow(progress, 2.2) * reach);
    let speed = pow(progress, 1.4);
    var mote = streak_at(center, mix(0.45, 1.3, speed), heading, 1.0 + speed * 9.0, corner);
    mote.tint = mix(tint(1u), tint(2u), step(0.8, random(star, 5u)));
    mote.strength = smoothstep(0.0, 0.25, progress) * mix(0.45, 1.0, random(star, 6u));
    return mote;
}
