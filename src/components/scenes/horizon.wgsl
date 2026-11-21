const HORIZON = 0.5;
const SUN_RADIUS = 0.2;

fn light_at(covering: Covering) -> vec3<f32> {
    let spot = spot_of(covering);
    let place = spot / scene.size.y;
    let middle = scene.size.x / scene.size.y * 0.5;
    let grid = scene.tint_lights[0].rgb;
    let sun = scene.tint_lights[1].rgb;
    let haze = scene.tint_lights[2].rgb;

    if place.y < HORIZON {
        let height = place.y / HORIZON;
        var sky = mix(scene.base_light.rgb, haze * 0.22, pow(height, 2.5));
        let center = vec2<f32>(middle, HORIZON - 0.03);
        let distance = length(place - center);
        let across = (place.y - (center.y - SUN_RADIUS)) / (SUN_RADIUS * 2.0);
        let band = fract(across * 9.0 - scene.seconds * 0.15);
        let gap = across > 0.45 && band < mix(0.0, 0.55, smoothstep(0.45, 1.0, across));
        let disc = (1.0 - smoothstep(SUN_RADIUS - 0.003, SUN_RADIUS + 0.003, distance)) * select(1.0, 0.0, gap);
        let face = mix(sun, grid, smoothstep(0.1, 1.0, across));
        sky = mix(sky, face * 0.4, disc);
        sky += sun * exp(-max(distance - SUN_RADIUS, 0.0) * 9.0) * 0.06;
        return sky + haze * exp(-(HORIZON - place.y) * 14.0) * 0.1;
    }

    let below = place.y - HORIZON;
    let depth = 0.4 / max(below, 0.0005);
    let pixel = 1.0 / scene.size.y;
    let sideways = (place.x - middle) * depth;
    let line_across = abs(fract(sideways * 1.4) - 0.5) / 1.4;
    let line_along = abs(fract(depth * 0.9 - scene.seconds * 0.55) - 0.5) / 0.9;
    let across_width = depth * pixel * 1.3;
    let along_width = 0.4 / (below * below) * pixel * 1.3;
    let lines = max(
        1.0 - smoothstep(across_width * 0.5, across_width * 1.5, line_across),
        1.0 - smoothstep(along_width * 0.5, along_width * 1.5, line_along),
    );
    let fade = smoothstep(0.0, 0.3, below);
    let floor_light = scene.base_light.rgb * 0.7 + haze * exp(-below * 10.0) * 0.12;
    return floor_light + grid * lines * fade * 0.45;
}
