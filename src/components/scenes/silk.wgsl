fn fold(spot: vec2<f32>) -> f32 {
    let time = scene.seconds * 0.06;
    let first = vec2<f32>(noise(spot + vec2<f32>(time, -time * 0.7)), noise(spot + vec2<f32>(5.2 - time * 0.5, 1.3 + time)));
    let second = vec2<f32>(noise(spot + first * 2.2 + vec2<f32>(1.7, 9.2) + time * 0.4), noise(spot + first * 2.2 + vec2<f32>(8.3, 2.8) - time * 0.3));
    return noise(spot + second * 2.0);
}

fn glow(spot: vec2<f32>) -> vec3<f32> {
    let place = spot / scene.size.y * 2.2;
    let value = fold(place);
    let sheen = pow(1.0 - abs(fold(place + vec2<f32>(0.02, 0.0)) - value) * 30.0, 6.0);
    let cloth = mix(mix(tint(0u), tint(1u), smoothstep(0.25, 0.6, value)), tint(2u), smoothstep(0.6, 0.85, value));
    let depth = 0.22 + 0.16 * smoothstep(0.3, 0.8, value);
    return mix(scene.base.rgb, cloth, depth) + tint(2u) * clamp(sheen, 0.0, 1.0) * 0.06;
}

fn light_at(covering: Covering) -> vec3<f32> {
    return sampled_glow(covering);
}
