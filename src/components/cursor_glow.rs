use super::imported_theme::brightness;
use super::style::luminance;
use crate::quadra::Message;
use crate::state::Animations;
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

const RADIUS: f32 = 230.0;
const FOLLOWING_SPEED: f32 = 10.0;
const FADING_TIME: f32 = 0.35;
const FULL_FRAME: Duration = Duration::from_millis(33);
const REDUCED_FRAME: Duration = Duration::from_millis(66);
const SETTLED_DISTANCE: f32 = 0.5;
const LONGEST_STEP: f32 = 0.1;
const DARK_WINDOW_STRENGTH: f32 = 0.16;
const DARK_WINDOW_LIGHT: f32 = 0.05;
const LIGHT_WINDOW_STRENGTH: f32 = 0.1;
const LIGHT_WINDOW_BRIGHTNESS: f32 = 0.55;
const SHADER: &str = r#"
struct Glow {
    center: vec2<f32>,
    size: vec2<f32>,
    linear_color: vec4<f32>,
    srgb_color: vec4<f32>,
    strength: f32,
    radius: f32,
}

struct Reach {
    @builtin(position) position: vec4<f32>,
    @location(0) offset: vec2<f32>,
}

@group(0) @binding(0) var<uniform> glow: Glow;

@vertex
fn around(@builtin(vertex_index) corner: u32) -> Reach {
    let offset = vec2<f32>(f32(corner & 1u), f32(corner >> 1u)) * 2.0 - 1.0;
    let spot = glow.center + offset * glow.radius;
    var reach: Reach;
    reach.position = vec4<f32>(spot.x / glow.size.x * 2.0 - 1.0, 1.0 - spot.y / glow.size.y * 2.0, 0.0, 1.0);
    reach.offset = offset;
    return reach;
}

fn cover(reach: Reach) -> f32 {
    return exp(-dot(reach.offset, reach.offset) * 4.0) * glow.strength;
}

@fragment
fn light(reach: Reach) -> @location(0) vec4<f32> {
    let cover = cover(reach);
    return vec4<f32>(glow.srgb_color.rgb * cover, cover);
}

@fragment
fn light_on_srgb(reach: Reach) -> @location(0) vec4<f32> {
    let cover = cover(reach);
    return vec4<f32>(glow.linear_color.rgb * cover, cover);
}
"#;

pub fn cursor_glow(color: Color, window: Color, motion: Animations) -> Element<'static, Message> {
    let peak = if brightness(window) > LIGHT_WINDOW_BRIGHTNESS {
        LIGHT_WINDOW_STRENGTH
    } else {
        (DARK_WINDOW_LIGHT / luminance(color).max(f32::EPSILON)).min(DARK_WINDOW_STRENGTH)
    };
    Element::new(GlowLayer {
        glow: shader::Shader::new(CursorGlow {
            color,
            peak,
            motion,
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into(),
    })
}

struct GlowLayer {
    glow: Element<'static, Message>,
}

fn is_lit(tree: &Tree) -> bool {
    match &tree.state {
        tree::State::Some(state) => state
            .downcast_ref::<Follower>()
            .is_some_and(|follower| follower.strength > 0.0),
        tree::State::None => false,
    }
}

impl Widget<Message, Theme, Renderer> for GlowLayer {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.glow)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.glow));
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let glow = self
            .glow
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits);
        layout::Node::with_children(glow.size(), vec![glow])
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
        let (Renderer::Primary(_), Some(glow_layout)) = (renderer, layout.children().next()) else {
            return;
        };
        self.glow.as_widget_mut().update(
            &mut tree.children[0],
            event,
            glow_layout,
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
        let Some(glow_layout) = layout.children().next() else {
            return;
        };
        if matches!(renderer, Renderer::Primary(_)) && is_lit(&tree.children[0]) {
            self.glow.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                glow_layout,
                cursor,
                viewport,
            );
        }
    }
}

struct CursorGlow {
    color: Color,
    peak: f32,
    motion: Animations,
}

#[derive(Default)]
struct Follower {
    target: Option<Point>,
    shown: Point,
    strength: f32,
    last_frame: Option<Instant>,
}

impl Follower {
    fn is_resting(&self) -> bool {
        self.last_frame.is_none()
    }

    fn has_somewhere_to_go(&self) -> bool {
        let wanted = if self.target.is_some() { 1.0 } else { 0.0 };
        self.strength != wanted
            || self
                .target
                .is_some_and(|target| target.distance(self.shown) >= SETTLED_DISTANCE)
    }

    fn aim_at(&mut self, target: Option<Point>) -> bool {
        if self.target.is_none()
            && self.strength == 0.0
            && let Some(target) = target
        {
            self.shown = target;
        }
        self.target = target;
        self.is_resting()
    }

    fn advance(&mut self, now: Instant, motion: Animations) -> Option<Instant> {
        let step = self.last_frame.map_or(0.0, |last_frame| {
            now.saturating_duration_since(last_frame)
                .as_secs_f32()
                .min(LONGEST_STEP)
        });
        if let Some(target) = self.target {
            let closing = if motion == Animations::Full {
                1.0 - (-step * FOLLOWING_SPEED).exp()
            } else {
                1.0
            };
            self.shown = self.shown + (target - self.shown) * closing;
        }
        let wanted = if self.target.is_some() { 1.0 } else { 0.0 };
        let change = step / FADING_TIME;
        self.strength = if self.strength < wanted {
            (self.strength + change).min(wanted)
        } else {
            (self.strength - change).max(wanted)
        };

        let has_arrived = self
            .target
            .is_none_or(|target| target.distance(self.shown) < SETTLED_DISTANCE);
        if has_arrived && self.strength == wanted {
            self.last_frame = None;
            return None;
        }
        self.last_frame = Some(now);
        let frame = if motion == Animations::Full {
            FULL_FRAME
        } else {
            REDUCED_FRAME
        };
        Some(now + frame)
    }
}

impl shader::Program<Message> for CursorGlow {
    type State = Follower;
    type Primitive = GlowFrame;

    fn update(
        &self,
        follower: &mut Follower,
        event: &Event,
        _bounds: Rectangle,
        _cursor: Cursor,
    ) -> Option<shader::Action<Message>> {
        let wakes = match event {
            Event::Window(window::Event::RedrawRequested(now)) => {
                if follower.is_resting() && !follower.has_somewhere_to_go() {
                    return None;
                }
                return follower
                    .advance(*now, self.motion)
                    .map(shader::Action::request_redraw_at);
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                follower.aim_at(Some(*position))
            }
            Event::Mouse(mouse::Event::CursorLeft) | Event::Window(window::Event::Unfocused) => {
                follower.aim_at(None)
            }
            _ => false,
        };
        wakes.then(shader::Action::request_redraw)
    }

    fn draw(&self, follower: &Follower, _cursor: Cursor, bounds: Rectangle) -> GlowFrame {
        GlowFrame {
            center: follower.shown - Vector::new(bounds.x, bounds.y),
            strength: follower.strength * self.peak,
            color: self.color,
        }
    }
}

#[derive(Debug)]
struct GlowFrame {
    center: Point,
    strength: f32,
    color: Color,
}

impl shader::Primitive for GlowFrame {
    type Pipeline = GlowPipeline;

    fn prepare(
        &self,
        pipeline: &mut GlowPipeline,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        bounds: &Rectangle,
        _viewport: &Viewport,
    ) {
        let [red, green, blue, _] = self.color.into_linear();
        let uniforms: [f32; 16] = [
            self.center.x,
            self.center.y,
            bounds.width,
            bounds.height,
            red,
            green,
            blue,
            1.0,
            self.color.r,
            self.color.g,
            self.color.b,
            1.0,
            self.strength,
            RADIUS,
            0.0,
            0.0,
        ];
        queue.write_buffer(&pipeline.uniforms, 0, bytemuck::cast_slice(&uniforms));
    }

    fn draw(&self, pipeline: &GlowPipeline, render_pass: &mut wgpu::RenderPass<'_>) -> bool {
        if self.strength <= 0.0 {
            return true;
        }
        render_pass.set_pipeline(&pipeline.pipeline);
        render_pass.set_bind_group(0, &pipeline.group, &[]);
        render_pass.draw(0..4, 0..1);
        true
    }
}

struct GlowPipeline {
    pipeline: wgpu::RenderPipeline,
    uniforms: wgpu::Buffer,
    group: wgpu::BindGroup,
}

impl shader::Pipeline for GlowPipeline {
    fn new(device: &wgpu::Device, _queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("cursor glow"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("cursor glow"),
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
            label: Some("cursor glow"),
            size: std::mem::size_of::<[f32; 16]>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("cursor glow"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms.as_entire_binding(),
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("cursor glow"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("cursor glow"),
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
    use super::{Follower, SHADER};
    use crate::state::Animations;
    use iced::Point;
    use iced::time::{Duration, Instant};

    #[test]
    fn follows_the_cursor_then_rests() {
        let start = Instant::now();
        let mut follower = Follower::default();
        assert!(follower.aim_at(Some(Point::new(100.0, 80.0))));
        follower.aim_at(Some(Point::new(300.0, 200.0)));
        let mut now = start;
        let mut frames = 0;
        while let Some(next) = follower.advance(now, Animations::Full) {
            now = next;
            frames += 1;
            assert!(frames < 200);
        }
        assert!(follower.shown.distance(Point::new(300.0, 200.0)) < 1.0);
        assert_eq!(follower.strength, 1.0);
        assert!(now - start < Duration::from_secs(3));
    }

    #[test]
    fn a_resting_glow_asks_for_no_frames() {
        let mut follower = Follower::default();
        assert!(!follower.has_somewhere_to_go());
        follower.aim_at(Some(Point::new(10.0, 10.0)));
        assert!(follower.has_somewhere_to_go());
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
