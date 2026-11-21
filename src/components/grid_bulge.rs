use crate::quadra::Message;
use iced::mouse::{self, Cursor};
use iced::time::{Duration, Instant};
use iced::wgpu;
use iced::widget::shader::{self, Viewport};
use iced::{Color, Element, Event, Length, Point, Rectangle, Vector, window};

const EASING_TIME: f32 = 0.25;
const EASING_FRAME: Duration = Duration::from_millis(16);
const LONGEST_STEP: f32 = 0.1;
const UNIFORM_FLOATS: usize = 16;
const SHADER: &str = r#"
struct Grid {
    pointer: vec2<f32>,
    size: vec2<f32>,
    background: vec4<f32>,
    line: vec4<f32>,
    accent: vec4<f32>,
}

struct Covering {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

const CELL_SIZE = 32.0;
const REACH = 150.0;
const PULL = 0.38;

@group(0) @binding(0) var<uniform> grid: Grid;

@vertex
fn cover(@builtin(vertex_index) corner: u32) -> Covering {
    let spot = vec2<f32>(f32((corner << 1u) & 2u), f32(corner & 2u));
    var covering: Covering;
    covering.position = vec4<f32>(spot.x * 2.0 - 1.0, 1.0 - spot.y * 2.0, 0.0, 1.0);
    covering.uv = spot;
    return covering;
}

fn linear_from_srgb(color: vec3<f32>) -> vec3<f32> {
    let low = color / 12.92;
    let high = pow((color + 0.055) / 1.055, vec3<f32>(2.4));
    return select(high, low, color <= vec3<f32>(0.04045));
}

fn painted(covering: Covering) -> vec3<f32> {
    let spot = covering.uv * grid.size;
    let away = spot - grid.pointer;
    let closeness = exp(-dot(away, away) / (REACH * REACH)) * grid.accent.a;
    let source = spot - away * closeness * PULL;
    let cells = (source - vec2<f32>(0.5)) / CELL_SIZE;
    let width = max(fwidth(cells), vec2<f32>(0.0001));
    let gap = min(fract(cells), 1.0 - fract(cells));
    let lines = 1.0 - smoothstep(width * 0.5, width * 1.5, gap);
    let on_line = max(lines.x, lines.y);
    let line_color = mix(grid.line.rgb, grid.accent.rgb, closeness * 0.6);
    let line_alpha = grid.line.a + closeness * 0.22;
    return mix(grid.background.rgb, line_color, on_line * line_alpha);
}

@fragment
fn show(covering: Covering) -> @location(0) vec4<f32> {
    return vec4<f32>(painted(covering), 1.0);
}

@fragment
fn show_on_srgb(covering: Covering) -> @location(0) vec4<f32> {
    return vec4<f32>(linear_from_srgb(painted(covering)), 1.0);
}
"#;

pub fn grid_bulge(background: Color, line: Color, accent: Color) -> Element<'static, Message> {
    shader::Shader::new(GridBulge {
        background,
        line,
        accent,
    })
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

struct GridBulge {
    background: Color,
    line: Color,
    accent: Color,
}

#[derive(Default)]
struct Bulge {
    pointer: Option<Point>,
    last_place: Point,
    strength: f32,
    last_frame: Option<Instant>,
}

impl Bulge {
    fn wanted_strength(&self) -> f32 {
        if self.pointer.is_some() { 1.0 } else { 0.0 }
    }

    fn has_somewhere_to_go(&self) -> bool {
        self.strength != self.wanted_strength()
    }

    fn aim_at(&mut self, pointer: Option<Point>) {
        if let Some(place) = pointer {
            self.last_place = place;
        }
        self.pointer = pointer;
    }

    fn advance(&mut self, now: Instant) -> Option<Instant> {
        let step = self.last_frame.map_or(0.0, |last_frame| {
            now.saturating_duration_since(last_frame)
                .as_secs_f32()
                .min(LONGEST_STEP)
        });
        let wanted = self.wanted_strength();
        let change = step / EASING_TIME;
        self.strength = if self.strength < wanted {
            (self.strength + change).min(wanted)
        } else {
            (self.strength - change).max(wanted)
        };
        if self.strength == wanted {
            self.last_frame = None;
            return None;
        }
        self.last_frame = Some(now);
        Some(now + EASING_FRAME)
    }
}

impl shader::Program<Message> for GridBulge {
    type State = Bulge;
    type Primitive = GridFrame;

    fn update(
        &self,
        bulge: &mut Bulge,
        event: &Event,
        _bounds: Rectangle,
        _cursor: Cursor,
    ) -> Option<shader::Action<Message>> {
        match event {
            Event::Window(window::Event::RedrawRequested(now)) => {
                if bulge.last_frame.is_none() && !bulge.has_somewhere_to_go() {
                    return None;
                }
                bulge.advance(*now).map(shader::Action::request_redraw_at)
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                bulge.aim_at(Some(*position));
                Some(shader::Action::request_redraw())
            }
            Event::Mouse(mouse::Event::CursorLeft) | Event::Window(window::Event::Unfocused) => {
                bulge.aim_at(None);
                Some(shader::Action::request_redraw())
            }
            _ => None,
        }
    }

    fn draw(&self, bulge: &Bulge, _cursor: Cursor, bounds: Rectangle) -> GridFrame {
        GridFrame {
            pointer: bulge.last_place - Vector::new(bounds.x, bounds.y),
            strength: bulge.strength,
            background: self.background,
            line: self.line,
            accent: self.accent,
        }
    }
}

#[derive(Debug)]
struct GridFrame {
    pointer: Point,
    strength: f32,
    background: Color,
    line: Color,
    accent: Color,
}

impl GridFrame {
    fn uniforms(&self, bounds: &Rectangle) -> [f32; UNIFORM_FLOATS] {
        [
            self.pointer.x,
            self.pointer.y,
            bounds.width,
            bounds.height,
            self.background.r,
            self.background.g,
            self.background.b,
            1.0,
            self.line.r,
            self.line.g,
            self.line.b,
            self.line.a,
            self.accent.r,
            self.accent.g,
            self.accent.b,
            self.strength,
        ]
    }
}

impl shader::Primitive for GridFrame {
    type Pipeline = GridPipeline;

    fn prepare(
        &self,
        pipeline: &mut GridPipeline,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        bounds: &Rectangle,
        _viewport: &Viewport,
    ) {
        queue.write_buffer(
            &pipeline.uniforms,
            0,
            bytemuck::cast_slice(&self.uniforms(bounds)),
        );
    }

    fn draw(&self, pipeline: &GridPipeline, render_pass: &mut wgpu::RenderPass<'_>) -> bool {
        render_pass.set_pipeline(&pipeline.pipeline);
        render_pass.set_bind_group(0, &pipeline.group, &[]);
        render_pass.draw(0..3, 0..1);
        true
    }
}

struct GridPipeline {
    pipeline: wgpu::RenderPipeline,
    uniforms: wgpu::Buffer,
    group: wgpu::BindGroup,
}

impl shader::Pipeline for GridPipeline {
    fn new(device: &wgpu::Device, _queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("grid bulge"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("grid bulge"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("grid bulge"),
            size: std::mem::size_of::<[f32; UNIFORM_FLOATS]>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("grid bulge"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms.as_entire_binding(),
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("grid bulge"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("grid bulge"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("cover"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some(if format.is_srgb() {
                    "show_on_srgb"
                } else {
                    "show"
                }),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
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
    use super::{Bulge, SHADER};
    use iced::Point;
    use iced::time::Instant;

    #[test]
    fn bulges_in_then_rests() {
        let mut bulge = Bulge::default();
        bulge.aim_at(Some(Point::new(40.0, 40.0)));
        let mut now = Instant::now();
        let mut frames = 0;
        while let Some(next) = bulge.advance(now) {
            now = next;
            frames += 1;
            assert!(frames < 60);
        }
        assert_eq!(bulge.strength, 1.0);
        assert!(!bulge.has_somewhere_to_go());
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
