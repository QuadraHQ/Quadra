struct Scene {
    size: vec2<f32>,
    seconds: f32,
    presence: f32,
    base: vec4<f32>,
    tints: array<vec4<f32>, 3>,
    base_light: vec4<f32>,
    tint_lights: array<vec4<f32>, 3>,
}

struct Covering {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

struct Mote {
    @builtin(position) position: vec4<f32>,
    @location(0) offset: vec2<f32>,
    @location(1) tint: vec3<f32>,
    @location(2) strength: f32,
}

const MOTE_REACH = 4.0;
const DISC_REACH = 1.1;
const TAU = 6.2831855;
const SRGB_STEP_PER_ROOT = 2.4 / 1.055 / 255.0;

@group(0) @binding(0) var<uniform> scene: Scene;
@group(1) @binding(0) var glow_texture: texture_2d<f32>;
@group(1) @binding(1) var smoothly: sampler;

@vertex
fn cover(@builtin(vertex_index) corner: u32) -> Covering {
    let spot = vec2<f32>(f32((corner << 1u) & 2u), f32(corner & 2u));
    var covering: Covering;
    covering.position = vec4<f32>(spot.x * 2.0 - 1.0, 1.0 - spot.y * 2.0, 0.0, 1.0);
    covering.uv = spot;
    return covering;
}

fn hash(seed: vec2<f32>) -> f32 {
    var mixed = fract(seed.xyx * 0.1031);
    mixed += dot(mixed, mixed.yzx + 33.33);
    return fract((mixed.x + mixed.y) * mixed.z);
}

fn noise(spot: vec2<f32>) -> f32 {
    let cell = floor(spot);
    let within = fract(spot);
    let eased = within * within * (3.0 - 2.0 * within);
    let top = mix(hash(cell), hash(cell + vec2<f32>(1.0, 0.0)), eased.x);
    let bottom = mix(hash(cell + vec2<f32>(0.0, 1.0)), hash(cell + vec2<f32>(1.0, 1.0)), eased.x);
    return mix(top, bottom, eased.y);
}

fn scramble(value: u32) -> u32 {
    let state = value * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

fn random(index: u32, salt: u32) -> f32 {
    return f32(scramble(index ^ scramble(salt)) >> 8u) / 16777216.0;
}

fn linear_from_srgb(color: vec3<f32>) -> vec3<f32> {
    let low = color / 12.92;
    let high = pow((color + 0.055) / 1.055, vec3<f32>(2.4));
    return select(high, low, color <= vec3<f32>(0.04045));
}

fn srgb_from_linear(color: vec3<f32>) -> vec3<f32> {
    let low = color * 12.92;
    let high = 1.055 * pow(color, vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(high, low, color <= vec3<f32>(0.0031308));
}

fn spot_of(covering: Covering) -> vec2<f32> {
    return covering.uv * scene.size;
}

fn finished(covering: Covering) -> vec3<f32> {
    let pixel = floor(covering.position.xy);
    let color = mix(scene.base_light.rgb, max(light_at(covering), vec3<f32>(0.0)), scene.presence);
    return max(color + (hash(pixel) - 0.5) * sqrt(color) * SRGB_STEP_PER_ROOT, vec3<f32>(0.0));
}

fn mote_at(center: vec2<f32>, radius: f32, corner: u32) -> Mote {
    let offset = (vec2<f32>(f32(corner & 1u), f32(corner >> 1u)) * 2.0 - 1.0) * MOTE_REACH;
    let spot = center + offset * radius;
    var mote: Mote;
    mote.position = vec4<f32>(spot.x / scene.size.x * 2.0 - 1.0, 1.0 - spot.y / scene.size.y * 2.0, 0.0, 1.0);
    mote.offset = offset;
    return mote;
}

fn streak_at(center: vec2<f32>, radius: f32, direction: vec2<f32>, stretch: f32, corner: u32) -> Mote {
    let offset = (vec2<f32>(f32(corner & 1u), f32(corner >> 1u)) * 2.0 - 1.0) * MOTE_REACH;
    let across = vec2<f32>(-direction.y, direction.x);
    let spot = center + (direction * offset.x * stretch + across * offset.y) * radius;
    var mote: Mote;
    mote.position = vec4<f32>(spot.x / scene.size.x * 2.0 - 1.0, 1.0 - spot.y / scene.size.y * 2.0, 0.0, 1.0);
    mote.offset = offset;
    return mote;
}

fn disc_at(center: vec2<f32>, radius: f32, corner: u32) -> Mote {
    let offset = (vec2<f32>(f32(corner & 1u), f32(corner >> 1u)) * 2.0 - 1.0) * DISC_REACH;
    let spot = center + offset * radius;
    var mote: Mote;
    mote.position = vec4<f32>(spot.x / scene.size.x * 2.0 - 1.0, 1.0 - spot.y / scene.size.y * 2.0, 0.0, 1.0);
    mote.offset = offset;
    return mote;
}

fn disc_light(mote: Mote) -> f32 {
    let distance = length(mote.offset);
    let body = 1.0 - smoothstep(0.88, 1.0, distance);
    return body * (0.7 + 0.3 * smoothstep(0.5, 0.95, distance)) * mote.strength * scene.presence;
}

fn tint(index: u32) -> vec3<f32> {
    return scene.tints[index].rgb;
}

fn mote_light(mote: Mote) -> f32 {
    let distance_squared = dot(mote.offset, mote.offset);
    return (exp(-distance_squared) + exp(-distance_squared / 6.0) * 0.2) * mote.strength * scene.presence;
}

@fragment
fn show(covering: Covering) -> @location(0) vec4<f32> {
    return vec4<f32>(srgb_from_linear(finished(covering)), 1.0);
}

@fragment
fn show_on_srgb(covering: Covering) -> @location(0) vec4<f32> {
    return vec4<f32>(finished(covering), 1.0);
}

@fragment
fn light_only(covering: Covering) -> @location(0) vec4<f32> {
    return vec4<f32>(max(srgb_from_linear(finished(covering)) - scene.base.rgb, vec3<f32>(0.0)), 0.0);
}

@fragment
fn light_only_on_srgb(covering: Covering) -> @location(0) vec4<f32> {
    return vec4<f32>(max(finished(covering) - scene.base_light.rgb, vec3<f32>(0.0)), 0.0);
}

@fragment
fn shine(mote: Mote) -> @location(0) vec4<f32> {
    return vec4<f32>(mote.tint * mote_light(mote), 0.0);
}

@fragment
fn shine_on_srgb(mote: Mote) -> @location(0) vec4<f32> {
    return vec4<f32>(linear_from_srgb(mote.tint) * mote_light(mote), 0.0);
}

@fragment
fn shine_disc(mote: Mote) -> @location(0) vec4<f32> {
    return vec4<f32>(mote.tint * disc_light(mote), 0.0);
}

@fragment
fn shine_disc_on_srgb(mote: Mote) -> @location(0) vec4<f32> {
    return vec4<f32>(linear_from_srgb(mote.tint) * disc_light(mote), 0.0);
}

fn ink(tint: vec3<f32>, light: f32) -> vec4<f32> {
    let cover = clamp(light * 0.75, 0.0, 0.8);
    return vec4<f32>(tint * 0.5 * cover, cover);
}

@fragment
fn shade(mote: Mote) -> @location(0) vec4<f32> {
    return ink(mote.tint, mote_light(mote));
}

@fragment
fn shade_on_srgb(mote: Mote) -> @location(0) vec4<f32> {
    return ink(linear_from_srgb(mote.tint), mote_light(mote));
}

@fragment
fn shade_disc(mote: Mote) -> @location(0) vec4<f32> {
    return ink(mote.tint, disc_light(mote) * 2.5);
}

@fragment
fn shade_disc_on_srgb(mote: Mote) -> @location(0) vec4<f32> {
    return ink(linear_from_srgb(mote.tint), disc_light(mote) * 2.5);
}
