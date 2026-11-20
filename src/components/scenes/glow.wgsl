fn sampled_glow(covering: Covering) -> vec3<f32> {
    let root = textureSample(glow_texture, smoothly, covering.uv).rgb;
    return root * root;
}

@fragment
fn paint_glow(covering: Covering) -> @location(0) vec4<f32> {
    return vec4<f32>(sqrt(linear_from_srgb(glow(spot_of(covering)))), 1.0);
}
