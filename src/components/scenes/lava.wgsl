fn blob(index: f32, spot: vec2<f32>) -> f32 {
    let time = scene.seconds;
    let speed = 0.05 + fract(index * 0.618) * 0.05;
    let rise = 0.5 + 0.5 * sin(time * speed * TAU * 0.4 + index * 2.4);
    let center = vec2<f32>(
        (0.12 + fract(index * 0.381) * 0.76) * scene.size.x / scene.size.y + 0.06 * sin(time * 0.13 + index),
        1.15 - rise * 1.3,
    );
    let size = 0.07 + fract(index * 0.7548) * 0.07;
    let offset = spot - center;
    return size * size / max(dot(offset, offset), 0.0001);
}

fn glow(spot: vec2<f32>) -> vec3<f32> {
    let place = spot / scene.size.y;
    var field = 0.0;
    for (var index = 0; index < 7; index++) {
        field += blob(f32(index), place);
    }
    let down = spot.y / scene.size.y;
    let wax = smoothstep(0.85, 1.15, field);
    let rim = smoothstep(0.45, 0.95, field) * (1.0 - wax);
    let heat = mix(tint(1u), tint(0u), smoothstep(0.1, 0.9, down));
    let lamp = mix(scene.base.rgb, mix(scene.base.rgb, tint(2u), 0.25), down);
    return mix(lamp, heat * 0.85, wax * 0.4) + heat * rim * 0.1;
}

fn light_at(covering: Covering) -> vec3<f32> {
    return sampled_glow(covering);
}
