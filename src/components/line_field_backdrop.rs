use crate::quadra::Message;
use crate::state::Animations;
use iced::mouse::Cursor;
use iced::time::Instant;
use iced::wgpu;
use iced::widget::shader::{self, Viewport};
use iced::{Element, Event, Length, Rectangle, window};
use std::f64::consts::TAU;
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const WIDTH: usize = 1280;
const HEIGHT: usize = 800;
const WANDERERS: usize = 520;
const SHORTEST_LIFE: i32 = 60;
const LONGEST_LIFE: i32 = 260;
const STEP_LENGTH: f64 = 1.15;
const TICK_INTERVAL: Duration = Duration::from_millis(40);
const TICKS_UNTIL_DRAWN: u32 = 190;
const STEPS_PER_TICK: u32 = 3;
const BRIGHTNESS_PER_VISIT: f32 = 15.0;
const BRIGHTEST_LINE: f32 = 112.0;
const FLOW_CELL_SIZE: f64 = 150.0;
const FLOW_TURNS: f64 = 2.0;
const FIRST_VISIT_CAPACITY: u64 = 64 * 1024;

static NEXT_FIELD: AtomicU64 = AtomicU64::new(1);

struct Random(u64);

impl Random {
    fn seeded_by_clock() -> Self {
        let nanoseconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_nanos() as u64);
        Self(nanoseconds ^ (u64::from(std::process::id()) << 32))
    }

    fn fraction(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut mixed = self.0;
        mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        mixed ^= mixed >> 31;
        (mixed >> 11) as f64 / (1u64 << 53) as f64
    }
}

struct Wanderer {
    x: f64,
    y: f64,
    steps_left: i32,
}

impl Wanderer {
    fn spawn(random: &mut Random) -> Self {
        let lifetimes = f64::from(LONGEST_LIFE - SHORTEST_LIFE + 1);
        Self {
            x: random.fraction() * WIDTH as f64,
            y: random.fraction() * HEIGHT as f64,
            steps_left: SHORTEST_LIFE + (random.fraction() * lifetimes) as i32,
        }
    }

    fn is_inside(&self) -> bool {
        (0.0..WIDTH as f64).contains(&self.x) && (0.0..HEIGHT as f64).contains(&self.y)
    }
}

struct FlowField {
    lattice: Vec<f64>,
    columns: usize,
}

impl FlowField {
    fn random(random: &mut Random) -> Self {
        let columns = (WIDTH as f64 / FLOW_CELL_SIZE) as usize + 3;
        let rows = (HEIGHT as f64 / FLOW_CELL_SIZE) as usize + 3;
        let lattice = (0..columns * rows).map(|_| random.fraction()).collect();
        Self { lattice, columns }
    }

    fn angle_at(&self, x: f64, y: f64) -> f64 {
        let (cell_x, cell_y) = (x / FLOW_CELL_SIZE, y / FLOW_CELL_SIZE);
        let (column, row) = (cell_x as usize, cell_y as usize);
        let across = smoothstep(cell_x.fract());
        let down = smoothstep(cell_y.fract());

        let top_left = row * self.columns + column;
        let bottom_left = top_left + self.columns;
        let top = lerp(self.lattice[top_left], self.lattice[top_left + 1], across);
        let bottom = lerp(
            self.lattice[bottom_left],
            self.lattice[bottom_left + 1],
            across,
        );
        lerp(top, bottom, down) * TAU * FLOW_TURNS
    }
}

fn smoothstep(t: f64) -> f64 {
    t * t * (3.0 - 2.0 * t)
}

fn lerp(from: f64, to: f64, t: f64) -> f64 {
    from + (to - from) * t
}

pub fn line_field_backdrop(motion: Animations) -> Element<'static, Message> {
    shader::Shader::new(LineFieldBackdrop { motion })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

struct LineFieldBackdrop {
    motion: Animations,
}

pub struct LineField {
    next_tick: Option<Instant>,
    id: u64,
    flow: FlowField,
    wanderers: Vec<Wanderer>,
    random: Random,
    ticks: u32,
    visits: Arc<Vec<u32>>,
}

impl Default for LineField {
    fn default() -> Self {
        let mut random = Random::seeded_by_clock();
        Self {
            next_tick: None,
            id: NEXT_FIELD.fetch_add(1, Ordering::Relaxed),
            flow: FlowField::random(&mut random),
            wanderers: (0..WANDERERS)
                .map(|_| Wanderer::spawn(&mut random))
                .collect(),
            random,
            ticks: 0,
            visits: Arc::default(),
        }
    }
}

impl LineField {
    fn is_drawn(&self) -> bool {
        self.ticks >= TICKS_UNTIL_DRAWN
    }

    fn tick(&mut self) {
        if self.is_drawn() {
            return;
        }
        self.advance();
        if self.is_drawn() {
            self.wanderers = Vec::new();
            self.flow.lattice = Vec::new();
        }
    }

    fn draw_everything(&mut self) {
        while !self.is_drawn() {
            self.tick();
        }
    }

    fn next_frame(&mut self, now: Instant, motion: Animations) -> Option<Instant> {
        if self.is_drawn() {
            return None;
        }
        if motion != Animations::Full {
            self.draw_everything();
            return Some(now);
        }
        let due = *self.next_tick.get_or_insert(now);
        if now >= due {
            self.tick();
            self.next_tick = Some(due.max(now - TICK_INTERVAL) + TICK_INTERVAL);
        }
        self.next_tick.filter(|_| !self.is_drawn()).or(Some(now))
    }

    fn advance(&mut self) {
        let Self {
            flow,
            wanderers,
            random,
            visits,
            ..
        } = self;
        let visits = Arc::make_mut(visits);
        for wanderer in wanderers {
            for _ in 0..STEPS_PER_TICK {
                let angle = flow.angle_at(wanderer.x, wanderer.y);
                wanderer.x += angle.cos() * STEP_LENGTH;
                wanderer.y += angle.sin() * STEP_LENGTH;
                wanderer.steps_left -= 1;
                if wanderer.steps_left <= 0 || !wanderer.is_inside() {
                    *wanderer = Wanderer::spawn(random);
                    continue;
                }
                visits.push(wanderer.x as u32 | ((wanderer.y as u32) << 16));
            }
        }
        self.ticks += 1;
    }
}

impl shader::Program<Message> for LineFieldBackdrop {
    type State = LineField;
    type Primitive = LineFieldFrame;

    fn update(
        &self,
        field: &mut LineField,
        event: &Event,
        _bounds: Rectangle,
        _cursor: Cursor,
    ) -> Option<shader::Action<Message>> {
        let Event::Window(window::Event::RedrawRequested(now)) = event else {
            return None;
        };
        field
            .next_frame(*now, self.motion)
            .map(shader::Action::request_redraw_at)
    }

    fn draw(&self, field: &LineField, _cursor: Cursor, _bounds: Rectangle) -> LineFieldFrame {
        LineFieldFrame {
            field: field.id,
            visits: field.visits.clone(),
        }
    }
}

pub struct LineFieldFrame {
    field: u64,
    visits: Arc<Vec<u32>>,
}

impl fmt::Debug for LineFieldFrame {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LineFieldFrame")
            .field("field", &self.field)
            .field("visits", &self.visits.len())
            .finish()
    }
}

impl shader::Primitive for LineFieldFrame {
    type Pipeline = LineFieldPipeline;

    fn prepare(
        &self,
        pipeline: &mut LineFieldPipeline,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bounds: &Rectangle,
        _viewport: &Viewport,
    ) {
        let is_new_field = pipeline.field != self.field;
        if is_new_field {
            pipeline.field = self.field;
            pipeline.drawn_visits = 0;
        }

        let new_visits = &self.visits[pipeline.drawn_visits.min(self.visits.len())..];
        if is_new_field || !new_visits.is_empty() {
            pipeline.draw_visits(device, queue, new_visits, is_new_field);
            pipeline.drawn_visits = self.visits.len();
        }

        pipeline.fit_to(queue, bounds.width / bounds.height);
    }

    fn draw(&self, pipeline: &LineFieldPipeline, render_pass: &mut wgpu::RenderPass<'_>) -> bool {
        render_pass.set_pipeline(&pipeline.show);
        render_pass.set_bind_group(0, &pipeline.showing, &[]);
        render_pass.draw(0..3, 0..1);
        true
    }
}

pub struct LineFieldPipeline {
    field: u64,
    drawn_visits: usize,
    fitted_aspect: f32,
    accumulate: wgpu::RenderPipeline,
    show: wgpu::RenderPipeline,
    lines: wgpu::TextureView,
    showing: wgpu::BindGroup,
    fit: wgpu::Buffer,
    visits: wgpu::Buffer,
}

impl LineFieldPipeline {
    fn draw_visits(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        visits: &[u32],
        clear_first: bool,
    ) {
        let needed = std::mem::size_of_val(visits) as u64;
        if needed > self.visits.size() {
            self.visits = visit_buffer(device, needed.next_power_of_two());
        }
        if needed > 0 {
            queue.write_buffer(&self.visits, 0, bytemuck::cast_slice(visits));
        }

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("line field visits"),
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("line field visits"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.lines,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: if clear_first {
                            wgpu::LoadOp::Clear(wgpu::Color::BLACK)
                        } else {
                            wgpu::LoadOp::Load
                        },
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            if needed > 0 {
                pass.set_pipeline(&self.accumulate);
                pass.set_vertex_buffer(0, self.visits.slice(..needed));
                pass.draw(0..visits.len() as u32, 0..1);
            }
        }
        queue.submit([encoder.finish()]);
    }

    fn fit_to(&mut self, queue: &wgpu::Queue, aspect: f32) {
        if !aspect.is_finite() || aspect == self.fitted_aspect {
            return;
        }
        self.fitted_aspect = aspect;

        let field_aspect = WIDTH as f32 / HEIGHT as f32;
        let (scale_x, scale_y) = if aspect > field_aspect {
            (1.0, field_aspect / aspect)
        } else {
            (aspect / field_aspect, 1.0)
        };
        let fit = [
            scale_x,
            scale_y,
            (1.0 - scale_x) / 2.0,
            (1.0 - scale_y) / 2.0,
        ];
        queue.write_buffer(&self.fit, 0, bytemuck::cast_slice(&fit));
    }
}

impl shader::Pipeline for LineFieldPipeline {
    fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("line field"),
            source: wgpu::ShaderSource::Wgsl(shader_source().into()),
        });

        let lines = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("line field"),
                size: wgpu::Extent3d {
                    width: WIDTH as u32,
                    height: HEIGHT as u32,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default());

        let accumulate = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("line field visits"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("visit"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<u32>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Uint32],
                }],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::PointList,
                ..wgpu::PrimitiveState::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("brighten"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::R8Unorm,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::RED,
                })],
            }),
            multiview: None,
            cache: None,
        });

        let show = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("line field"),
            layout: None,
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
                entry_point: if format.is_srgb() {
                    Some("show_on_srgb")
                } else {
                    Some("show")
                },
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

        let fit = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("line field fit"),
            size: std::mem::size_of::<[f32; 4]>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&fit, 0, bytemuck::cast_slice(&[1.0f32, 1.0, 0.0, 0.0]));

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("line field"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..wgpu::SamplerDescriptor::default()
        });
        let showing = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("line field"),
            layout: &show.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&lines),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: fit.as_entire_binding(),
                },
            ],
        });

        Self {
            field: 0,
            drawn_visits: 0,
            fitted_aspect: f32::NAN,
            accumulate,
            show,
            lines,
            showing,
            fit,
            visits: visit_buffer(device, FIRST_VISIT_CAPACITY),
        }
    }
}

fn visit_buffer(device: &wgpu::Device, size: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("line field visits"),
        size,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn shader_source() -> String {
    let width = WIDTH as f32;
    let height = HEIGHT as f32;
    let step = BRIGHTNESS_PER_VISIT / BRIGHTEST_LINE;
    let brightest = BRIGHTEST_LINE / 255.0;
    format!(
        "
struct Fit {{ scale_and_offset: vec4<f32> }}

@group(0) @binding(0) var lines: texture_2d<f32>;
@group(0) @binding(1) var smoothly: sampler;
@group(0) @binding(2) var<uniform> fit: Fit;

@vertex
fn visit(@location(0) packed: u32) -> @builtin(position) vec4<f32> {{
    let x = f32(packed & 0xffffu) + 0.5;
    let y = f32(packed >> 16u) + 0.5;
    return vec4<f32>(x / {width:.1} * 2.0 - 1.0, 1.0 - y / {height:.1} * 2.0, 0.0, 1.0);
}}

@fragment
fn brighten() -> @location(0) vec4<f32> {{
    return vec4<f32>({step:.8}, 0.0, 0.0, 0.0);
}}

struct Covering {{
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}}

@vertex
fn cover(@builtin(vertex_index) corner: u32) -> Covering {{
    let spot = vec2<f32>(f32((corner << 1u) & 2u), f32(corner & 2u));
    var covering: Covering;
    covering.position = vec4<f32>(spot.x * 2.0 - 1.0, 1.0 - spot.y * 2.0, 0.0, 1.0);
    covering.uv = fit.scale_and_offset.zw + spot * fit.scale_and_offset.xy;
    return covering;
}}

fn brightness(uv: vec2<f32>) -> f32 {{
    return textureSample(lines, smoothly, uv).r * {brightest:.8};
}}

@fragment
fn show(covering: Covering) -> @location(0) vec4<f32> {{
    let level = brightness(covering.uv);
    return vec4<f32>(level, level, level, 1.0);
}}

@fragment
fn show_on_srgb(covering: Covering) -> @location(0) vec4<f32> {{
    let level = brightness(covering.uv);
    let linear = select(pow((level + 0.055) / 1.055, 2.4), level / 12.92, level <= 0.04045);
    return vec4<f32>(linear, linear, linear, 1.0);
}}
"
    )
}
