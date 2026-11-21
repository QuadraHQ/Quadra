fn crest(across: f32, layer: f32) -> f32 {
    let time = scene.seconds * (0.16 + layer * 0.05);
    return 0.6 + layer * 0.11
        + 0.04 * sin(across * (1.8 + layer * 0.5) + time + layer * 1.7)
        + 0.018 * sin(across * (4.3 - layer) - time * 1.6 + layer * 4.1)
        + 0.006 * sin(across * 11.0 + time * 2.7);
}

fn wave(base: vec3<f32>, spot: vec2<f32>, layer: u32) -> vec3<f32> {
    let across = spot.x / scene.size.y;
    let down = spot.y / scene.size.y;
    let surface = crest(across, f32(layer));
    let depth = down - surface;
    let body = smoothstep(-0.006, 0.006, depth);
    let foam = exp(-pow(depth / 0.012, 2.0)) * 0.35;
    let shading = 1.0 - clamp(depth * 1.4, 0.0, 0.45);
    let water = mix(base, tint(layer) * shading, 0.28 + 0.1 * f32(layer));
    return mix(base, water, body) + tint(layer) * foam * 0.25;
}

fn glow(spot: vec2<f32>) -> vec3<f32> {
    let down = spot.y / scene.size.y;
    var color = mix(scene.base.rgb, tint(0u), 0.06 * smoothstep(0.2, 1.0, down));
    color = wave(color, spot, 0u);
    color = wave(color, spot, 1u);
    color = wave(color, spot, 2u);
    return color;
}

fn light_at(covering: Covering) -> vec3<f32> {
    return sampled_glow(covering);
}
