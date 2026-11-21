use super::imported_theme::brightness;
use super::style::luminance;
use crate::quadra::Message;
use crate::state::{Animations, CursorEffect};
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::{Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell};
use iced::mouse::{self, Cursor};
use iced::time::{Duration, Instant};
use iced::wgpu;
use iced::widget::shader::{self, Viewport};
use iced::{
    Color, Element, Event, Length, Point, Rectangle, Renderer, Size, Theme, Vector, window,
};
use std::collections::VecDeque;

const RADIUS: f32 = 230.0;
const FADING_TIME: f32 = 0.35;
const FADING_FRAME: Duration = Duration::from_millis(33);
const MOVING_FRAME: Duration = Duration::from_millis(16);
const REDUCED_FRAME: Duration = Duration::from_millis(66);
const LONGEST_STEP: f32 = 0.1;
const DARK_WINDOW_STRENGTH: f32 = 0.16;
const DARK_WINDOW_LIGHT: f32 = 0.05;
const LIGHT_WINDOW_STRENGTH: f32 = 0.1;
const LIGHT_WINDOW_BRIGHTNESS: f32 = 0.55;
const MOST_MARKS: usize = 16;
const TRAIL_LIFE: Duration = Duration::from_millis(400);
const RIPPLE_LIFE: Duration = Duration::from_millis(650);
const TRAIL_SPACING: f32 = 0.035;
const UNIFORM_FLOATS: usize = 16 + MOST_MARKS * 4;
const SHADER: &str = r#"
struct Effect {
    center: vec2<f32>,
    size: vec2<f32>,
    linear_color: vec4<f32>,
    srgb_color: vec4<f32>,
    strength: f32,
    radius: f32,
    kind: f32,
    unused: f32,
    marks: array<vec4<f32>, 16>,
}

struct Reach {
    @builtin(position) position: vec4<f32>,
    @location(0) offset: vec2<f32>,
    @location(1) @interpolate(flat) life: f32,
    @location(2) @interpolate(flat) is_mark: f32,
}

const GLOW = 0.0;
const SPOTLIGHT = 1.0;
const RING = 2.0;
const RIPPLES = 3.0;
const TRAIL = 4.0;

@group(0) @binding(0) var<uniform> effect: Effect;

fn reach_of(kind: f32) -> f32 {
    if kind == RING {
        return max(effect.radius * 0.09, 14.0) + 6.0;
    }
    if kind == RIPPLES {
        return effect.radius * 0.45 + 4.0;
    }
    if kind == TRAIL {
        return max(effect.radius * 0.05, 4.0) + 2.0;
    }
    return effect.radius;
}

@vertex
fn around(@builtin(vertex_index) corner: u32, @builtin(instance_index) index: u32) -> Reach {
    let unit = vec2<f32>(f32(corner & 1u), f32(corner >> 1u)) * 2.0 - 1.0;
    var reach: Reach;
    var center = effect.center;
    var extent = vec2<f32>(reach_of(effect.kind));
    reach.life = 0.0;
    reach.is_mark = 0.0;
    if index > 0u {
        let mark = effect.marks[index - 1u];
        center = mark.xy;
        reach.life = mark.z;
        reach.is_mark = 1.0;
    } else if effect.kind == SPOTLIGHT {
        center = effect.size * 0.5;
        extent = effect.size * 0.5;
    }
    let spot = center + unit * extent;
    reach.position = vec4<f32>(spot.x / effect.size.x * 2.0 - 1.0, 1.0 - spot.y / effect.size.y * 2.0, 0.0, 1.0);
    reach.offset = spot - select(effect.center, center, index > 0u);
    return reach;
}

fn dither(position: vec2<f32>) -> f32 {
    var mixed = fract(position.xyx * 0.1031);
    mixed += dot(mixed, mixed.yzx + 33.33);
    return (fract((mixed.x + mixed.y) * mixed.z) - 0.5) / 255.0;
}

fn glow(distance: f32) -> f32 {
    let spread = distance / effect.radius;
    let edge = exp(-4.0);
    return max(exp(-4.0 * spread * spread) - edge, 0.0) / (1.0 - edge) * effect.strength;
}

fn line(distance: f32, at: f32, width: f32) -> f32 {
    return 1.0 - smoothstep(width * 0.5, width * 0.5 + 1.0, abs(distance - at));
}

fn cover(reach: Reach) -> f32 {
    let distance = length(reach.offset);
    if reach.is_mark > 0.5 {
        let fading = 1.0 - reach.life;
        if effect.kind == RIPPLES {
            let grown = 1.0 - (1.0 - reach.life) * (1.0 - reach.life);
            return line(distance, effect.radius * 0.45 * grown, 2.0) * fading * fading * 0.8;
        }
        let size = max(effect.radius * 0.05, 4.0) * fading;
        return (1.0 - smoothstep(size - 1.0, size + 1.0, distance)) * fading * 0.6;
    }
    let shown = effect.strength;
    if effect.kind == SPOTLIGHT {
        return smoothstep(effect.radius * 0.3, effect.radius * 1.1, distance) * 0.6 * shown;
    }
    if effect.kind == RING {
        let size = max(effect.radius * 0.09, 14.0);
        let halo = exp(-distance * distance / (size * size)) * 0.12;
        return (line(distance, size, 1.5) * 0.85 + halo) * shown;
    }
    if effect.kind == GLOW {
        return glow(distance);
    }
    return 0.0;
}

fn painted(reach: Reach, color: vec3<f32>) -> vec4<f32> {
    let amount = clamp(cover(reach) + dither(reach.position.xy), 0.0, 1.0);
    if amount <= 0.0 {
        discard;
    }
    if effect.kind == SPOTLIGHT && reach.is_mark < 0.5 {
        return vec4<f32>(0.0, 0.0, 0.0, amount);
    }
    return vec4<f32>(color * amount, amount);
}

@fragment
fn light(reach: Reach) -> @location(0) vec4<f32> {
    return painted(reach, effect.srgb_color.rgb);
}

@fragment
fn light_on_srgb(reach: Reach) -> @location(0) vec4<f32> {
    return painted(reach, effect.linear_color.rgb);
}
"#;

pub struct Pointer {
    pub effect: CursorEffect,
    pub size: f32,
    pub color: Color,
    pub window: Color,
}

pub fn cursor_effect(pointer: Pointer, motion: Animations) -> Option<Element<'static, Message>> {
    let effect = shown_effect(pointer.effect, motion)?;
    let peak = if effect != CursorEffect::Glow {
        1.0
    } else if brightness(pointer.window) > LIGHT_WINDOW_BRIGHTNESS {
        LIGHT_WINDOW_STRENGTH
    } else {
        (DARK_WINDOW_LIGHT / luminance(pointer.color).max(f32::EPSILON)).min(DARK_WINDOW_STRENGTH)
    };
    Some(Element::new(EffectLayer {
        effect: shader::Shader::new(CursorFollower {
            effect,
            radius: RADIUS * pointer.size,
            color: pointer.color,
            peak,
            motion,
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into(),
    }))
}

fn shown_effect(effect: CursorEffect, motion: Animations) -> Option<CursorEffect> {
    match (effect, motion) {
        (CursorEffect::None, _) | (_, Animations::Off) => None,
        (CursorEffect::Ripples | CursorEffect::Trail, Animations::Reduced) => None,
        (effect, _) => Some(effect),
    }
}

struct EffectLayer {
    effect: Element<'static, Message>,
}

fn is_visible(tree: &Tree) -> bool {
    match &tree.state {
        tree::State::Some(state) => state
            .downcast_ref::<Follower>()
            .is_some_and(Follower::is_visible),
        tree::State::None => false,
    }
}

impl Widget<Message, Theme, Renderer> for EffectLayer {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.effect)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.effect));
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let effect = self
            .effect
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits);
        layout::Node::with_children(effect.size(), vec![effect])
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let (Renderer::Primary(_), Some(effect_layout)) = (renderer, layout.children().next())
        else {
            return;
        };
        self.effect.as_widget_mut().update(
            &mut tree.children[0],
            event,
            effect_layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let Some(effect_layout) = layout.children().next() else {
            return;
        };
        if matches!(renderer, Renderer::Primary(_)) && is_visible(&tree.children[0]) {
            self.effect.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                effect_layout,
                cursor,
                viewport,
            );
        }
    }
}

struct CursorFollower {
    effect: CursorEffect,
    radius: f32,
    color: Color,
    peak: f32,
    motion: Animations,
}

#[derive(Debug, Clone, Copy)]
struct Mark {
    place: Point,
    born: Instant,
}

#[derive(Default)]
struct Follower {
    target: Option<Point>,
    strength: f32,
    marks: VecDeque<Mark>,
    last_frame: Option<Instant>,
}

impl Follower {
    fn is_resting(&self) -> bool {
        self.last_frame.is_none()
    }

    fn is_visible(&self) -> bool {
        self.strength > 0.0 || !self.marks.is_empty()
    }

    fn wanted_strength(&self) -> f32 {
        if self.target.is_some() { 1.0 } else { 0.0 }
    }

    fn has_somewhere_to_go(&self) -> bool {
        self.strength != self.wanted_strength() || !self.marks.is_empty()
    }

    fn aim_at(&mut self, target: Option<Point>) {
        self.target = target;
    }

    fn leave_mark(&mut self, place: Point, now: Instant) {
        if self.marks.len() == MOST_MARKS {
            self.marks.pop_front();
        }
        self.marks.push_back(Mark { place, born: now });
    }

    fn trail_to(&mut self, place: Point, spacing: f32, now: Instant) {
        let is_far_enough = self
            .marks
            .back()
            .is_none_or(|last| last.place.distance(place) >= spacing);
        if is_far_enough {
            self.leave_mark(place, now);
        }
    }

    fn advance(
        &mut self,
        now: Instant,
        effect: CursorEffect,
        motion: Animations,
    ) -> Option<Instant> {
        let step = self.last_frame.map_or(0.0, |last_frame| {
            now.saturating_duration_since(last_frame)
                .as_secs_f32()
                .min(LONGEST_STEP)
        });
        let wanted = self.wanted_strength();
        let change = if motion == Animations::Full {
            step / FADING_TIME
        } else {
            1.0
        };
        self.strength = if self.strength < wanted {
            (self.strength + change).min(wanted)
        } else {
            (self.strength - change).max(wanted)
        };
        let life = life_of(effect);
        self.marks
            .retain(|mark| now.saturating_duration_since(mark.born) < life);

        if self.strength == wanted && self.marks.is_empty() {
            self.last_frame = None;
            return None;
        }
        self.last_frame = Some(now);
        let frame = match motion {
            Animations::Full if !self.marks.is_empty() => MOVING_FRAME,
            Animations::Full => FADING_FRAME,
            _ => REDUCED_FRAME,
        };
        Some(now + frame)
    }
}

fn life_of(effect: CursorEffect) -> Duration {
    if effect == CursorEffect::Ripples {
        RIPPLE_LIFE
    } else {
        TRAIL_LIFE
    }
}

fn kind_of(effect: CursorEffect) -> f32 {
    match effect {
        CursorEffect::None | CursorEffect::Glow => 0.0,
        CursorEffect::Spotlight => 1.0,
        CursorEffect::Ring => 2.0,
        CursorEffect::Ripples => 3.0,
        CursorEffect::Trail => 4.0,
    }
}

impl shader::Program<Message> for CursorFollower {
    type State = Follower;
    type Primitive = EffectFrame;

    fn update(
        &self,
        follower: &mut Follower,
        event: &Event,
        _bounds: Rectangle,
        _cursor: Cursor,
    ) -> Option<shader::Action<Message>> {
        let was_resting = follower.is_resting();
        match event {
            Event::Window(window::Event::RedrawRequested(now)) => {
                if was_resting && !follower.has_somewhere_to_go() {
                    return None;
                }
                follower
                    .advance(*now, self.effect, self.motion)
                    .map(shader::Action::request_redraw_at)
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                follower.aim_at(Some(*position));
                if self.effect == CursorEffect::Trail {
                    follower.trail_to(*position, self.radius * TRAIL_SPACING, Instant::now());
                }
                Some(shader::Action::request_redraw())
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                if self.effect == CursorEffect::Ripples =>
            {
                let place = follower.target?;
                follower.leave_mark(place, Instant::now());
                Some(shader::Action::request_redraw())
            }
            Event::Mouse(mouse::Event::CursorLeft) | Event::Window(window::Event::Unfocused) => {
                follower.aim_at(None);
                was_resting.then(shader::Action::request_redraw)
            }
            _ => None,
        }
    }

    fn draw(&self, follower: &Follower, _cursor: Cursor, bounds: Rectangle) -> EffectFrame {
        let origin = Vector::new(bounds.x, bounds.y);
        let now = Instant::now();
        let life = life_of(self.effect).as_secs_f32();
        let marks = follower
            .marks
            .iter()
            .map(|mark| {
                let age = now.saturating_duration_since(mark.born).as_secs_f32() / life;
                (mark.place - origin, age.min(1.0))
            })
            .collect();
        EffectFrame {
            center: follower
                .target
                .map_or(Point::ORIGIN, |target| target - origin),
            strength: follower.strength * self.peak,
            radius: self.radius,
            kind: kind_of(self.effect),
            color: self.color,
            marks,
        }
    }
}

#[derive(Debug)]
struct EffectFrame {
    center: Point,
    strength: f32,
    radius: f32,
    kind: f32,
    color: Color,
    marks: Vec<(Point, f32)>,
}

impl EffectFrame {
    fn uniforms(&self, size: Size) -> [f32; UNIFORM_FLOATS] {
        let [red, green, blue, _] = self.color.into_linear();
        let mut uniforms = [0.0; UNIFORM_FLOATS];
        uniforms[..16].copy_from_slice(&[
            self.center.x,
            self.center.y,
            size.width,
            size.height,
            red,
            green,
            blue,
            1.0,
            self.color.r,
            self.color.g,
            self.color.b,
            1.0,
            self.strength,
            self.radius,
            self.kind,
            0.0,
        ]);
        for (slot, (place, age)) in uniforms[16..].chunks_exact_mut(4).zip(&self.marks) {
            slot.copy_from_slice(&[place.x, place.y, *age, 0.0]);
        }
        uniforms
    }

    fn is_blank(&self) -> bool {
        self.strength <= 0.0 && self.marks.is_empty()
    }
}

impl shader::Primitive for EffectFrame {
    type Pipeline = EffectPipeline;

    fn prepare(
        &self,
        pipeline: &mut EffectPipeline,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        bounds: &Rectangle,
        _viewport: &Viewport,
    ) {
        let uniforms = self.uniforms(bounds.size());
        queue.write_buffer(&pipeline.uniforms, 0, bytemuck::cast_slice(&uniforms));
    }

    fn draw(&self, pipeline: &EffectPipeline, render_pass: &mut wgpu::RenderPass<'_>) -> bool {
        if self.is_blank() {
            return true;
        }
        let marks = self.marks.len() as u32;
        render_pass.set_pipeline(&pipeline.pipeline);
        render_pass.set_bind_group(0, &pipeline.group, &[]);
        render_pass.draw(0..4, 0..1 + marks);
        true
    }
}

struct EffectPipeline {
    pipeline: wgpu::RenderPipeline,
    uniforms: wgpu::Buffer,
    group: wgpu::BindGroup,
}

impl shader::Pipeline for EffectPipeline {
    fn new(device: &wgpu::Device, _queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("cursor effect"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("cursor effect"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cursor effect"),
            size: std::mem::size_of::<[f32; UNIFORM_FLOATS]>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("cursor effect"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms.as_entire_binding(),
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("cursor effect"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("cursor effect"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("around"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..wgpu::PrimitiveState::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some(if format.is_srgb() {
                    "light_on_srgb"
                } else {
                    "light"
                }),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });
        Self {
            pipeline,
            uniforms,
            group,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Follower, SHADER, shown_effect};
    use crate::state::{Animations, CursorEffect};
    use iced::Point;
    use iced::time::{Duration, Instant};

    #[test]
    fn fades_in_then_rests() {
        let start = Instant::now();
        let mut follower = Follower::default();
        follower.aim_at(Some(Point::new(100.0, 80.0)));
        let mut now = start;
        let mut frames = 0;
        while let Some(next) = follower.advance(now, CursorEffect::Glow, Animations::Full) {
            now = next;
            frames += 1;
            assert!(frames < 100);
        }
        assert_eq!(follower.strength, 1.0);
        assert!(now - start < Duration::from_secs(1));
    }

    #[test]
    fn a_resting_effect_asks_for_no_frames() {
        let mut follower = Follower::default();
        assert!(!follower.has_somewhere_to_go());
        follower.aim_at(Some(Point::new(10.0, 10.0)));
        assert!(follower.has_somewhere_to_go());
    }

    #[test]
    fn marks_fade_away_then_rest() {
        let start = Instant::now();
        let mut follower = Follower::default();
        for step in 0..40 {
            follower.trail_to(Point::new(step as f32 * 10.0, 0.0), 8.0, start);
        }
        assert_eq!(follower.marks.len(), 16);
        let later = start + Duration::from_secs(1);
        follower.advance(later, CursorEffect::Trail, Animations::Full);
        assert!(follower.marks.is_empty());
        assert!(!follower.has_somewhere_to_go());
    }

    #[test]
    fn moving_effects_stop_when_motion_is_reduced() {
        assert_eq!(shown_effect(CursorEffect::Trail, Animations::Reduced), None);
        assert_eq!(
            shown_effect(CursorEffect::Glow, Animations::Reduced),
            Some(CursorEffect::Glow)
        );
        assert_eq!(shown_effect(CursorEffect::Ring, Animations::Off), None);
    }

    #[test]
    fn shader_is_valid_wgsl() {
        let module = naga::front::wgsl::parse_str(SHADER).unwrap();
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::default(),
        )
        .validate(&module)
        .unwrap();
    }
}
