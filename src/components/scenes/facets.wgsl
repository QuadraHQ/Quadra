const FACET_SIZE = 96.0;
const ROOT_THREE_HALVES = 0.8660254;

fn light_at(covering: Covering) -> vec3<f32> {
    let spot = spot_of(covering);
    let row = spot.y / (FACET_SIZE * ROOT_THREE_HALVES);
    let across = spot.x / FACET_SIZE - row * 0.5;
    let cell = floor(vec2<f32>(across, row));
    let within = fract(vec2<f32>(across, row));
    let is_upper = within.x + within.y > 1.0;
    let side = select(0.0, 1.0, is_upper);
    let facet = cell * 2.0 + vec2<f32>(side, side * 0.5);

    let center = vec2<f32>(
        (cell.x + (1.0 + side) / 3.0 + (cell.y + (1.0 + side) / 3.0) * 0.5) * FACET_SIZE,
        (cell.y + (1.0 + side) / 3.0) * FACET_SIZE * ROOT_THREE_HALVES,
    );
    let shade = noise(facet * 0.21 + vec2<f32>(scene.seconds * 0.045, scene.seconds * 0.03));
    let sweep_line = dot(center / scene.size.y, vec2<f32>(0.8, 0.6)) - scene.seconds * 0.07;
    let sweep = pow(0.5 + 0.5 * sin(sweep_line * TAU * 0.8), 6.0);

    let to_edge = min(min(within.x, within.y), abs(1.0 - within.x - within.y) * 0.7071) * FACET_SIZE * ROOT_THREE_HALVES;
    let edge = 1.0 - smoothstep(0.4, 1.4, to_edge);

    let base = scene.base_light.rgb;
    var color = mix(base, scene.tint_lights[0].rgb, 0.015 + 0.05 * shade * shade);
    color = mix(color, scene.tint_lights[1].rgb, sweep * (0.03 + 0.04 * shade));
    return mix(color, scene.tint_lights[2].rgb, edge * (0.04 + 0.1 * sweep));
}
