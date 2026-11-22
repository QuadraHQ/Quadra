fn turned(spot: vec2<f32>, angle: f32) -> vec2<f32> {
    let along = vec2<f32>(cos(angle), sin(angle));
    return vec2<f32>(along.x * spot.x - along.y * spot.y, along.y * spot.x + along.x * spot.y);
}

fn leaves(spot: vec2<f32>) -> f32 {
    let time = scene.seconds;
    let breeze = vec2<f32>(sin(time * 0.45 + spot.y * 0.004), cos(time * 0.37 + spot.x * 0.003)) * 14.0;
    let canopy = turned(spot + breeze, 0.6) / vec2<f32>(170.0, 110.0) + vec2<f32>(time * 0.01, 0.0);
    let flutter = vec2<f32>(sin(time * 1.1 + spot.x * 0.02), cos(time * 0.9 + spot.y * 0.02)) * 0.06;
    return noise(canopy) * 0.62 + noise(canopy * 2.4 + flutter + 5.3) * 0.38;
}

fn glow(spot: vec2<f32>) -> vec3<f32> {
    let aspect = scene.size.x / scene.size.y;
    let from_corner = distance(spot / scene.size.y, vec2<f32>(aspect, 0.0));
    let reach = 1.0 - smoothstep(0.15, 1.3, from_corner);
    let shade = smoothstep(0.48, 0.58, leaves(spot));
    let sunlit = mix(scene.base.rgb, tint(0u), 0.3);
    let shaded = mix(scene.base.rgb, tint(1u), 0.18);
    return mix(scene.base.rgb, mix(sunlit, shaded, shade), reach * 0.75);
}

fn light_at(covering: Covering) -> vec3<f32> {
    let grain = (hash(floor(covering.position.xy) * 1.37 + 11.0) - 0.5) * 0.03;
    return sampled_glow(covering) * (1.0 + grain);
}
