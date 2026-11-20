use crate::quadra::Message;
use iced::mouse::Cursor;
use iced::time::{Duration, Instant};
use iced::wgpu;
use iced::widget::shader::{self, Viewport};
use iced::{Element, Event, Length, Rectangle, window};
use std::thread;

const GLOW_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const LONGEST_STEP: Duration = Duration::from_millis(100);
const TIMELINE_LENGTH: Duration = Duration::from_secs(4 * 60 * 60);
const SETTLES_AFTER_NO_INPUT_FOR: Duration = Duration::from_secs(60);
const SETTLING_TIME: Duration = Duration::from_millis(2500);
const WAKING_TIME: Duration = Duration::from_millis(600);
const FADING_IN_TIME: Duration = Duration::from_millis(400);
const PRELUDE: &str = include_str!("scenes/prelude.wgsl");
const GLOW_PASS: &str = include_str!("scenes/glow.wgsl");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scene {
    Aurora,
    Ember,
    Paper,
    Phosphor,
}

impl Scene {
    const ALL: [Scene; 4] = [Self::Aurora, Self::Ember, Self::Paper, Self::Phosphor];
    const COUNT: usize = Self::ALL.len();

    pub fn view(self) -> Element<'static, Message> {
        shader::Shader::new(SceneBackdrop { scene: self })
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn frames_per_second(self) -> u32 {
        match self {
            Self::Paper => 15,
            Self::Aurora | Self::Ember | Self::Phosphor => 20,
        }
    }

    fn frame_interval(self) -> Duration {
        Duration::from_secs(1) / self.frames_per_second()
    }

    fn pixels_per_glow_texel(self) -> Option<f32> {
        match self {
            Self::Aurora => Some(4.0),
            Self::Ember => Some(6.0),
            Self::Paper => Some(8.0),
            Self::Phosphor => None,
        }
    }

    fn motes(self) -> u32 {
        match self {
            Self::Aurora => 70,
            Self::Ember => 72,
            Self::Paper | Self::Phosphor => 0,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Aurora => "aurora",
            Self::Ember => "ember",
            Self::Paper => "paper",
            Self::Phosphor => "phosphor",
        }
    }

    fn shader_source(self) -> String {
        let own = match self {
            Self::Aurora => include_str!("scenes/aurora.wgsl"),
            Self::Ember => include_str!("scenes/ember.wgsl"),
            Self::Paper => include_str!("scenes/paper.wgsl"),
            Self::Phosphor => include_str!("scenes/phosphor.wgsl"),
        };
        let glow_pass = if self.pixels_per_glow_texel().is_some() {
            GLOW_PASS
        } else {
            ""
        };
        format!("{PRELUDE}\n{glow_pass}\n{own}")
    }
}

struct SceneBackdrop {
    scene: Scene,
}

#[derive(Default)]
struct Clock {
    shown: Duration,
    pace: f32,
    last_frame: Option<Instant>,
    next_frame: Option<Instant>,
    last_input: Option<Instant>,
    is_unfocused: bool,
}

impl Clock {
    fn seconds(&self) -> f32 {
        (self.shown.as_secs_f64() % TIMELINE_LENGTH.as_secs_f64()) as f32
    }

    fn is_resting(&self) -> bool {
        self.last_frame.is_none()
    }

    fn wants_motion(&self, now: Instant) -> bool {
        !self.is_unfocused
            && self.last_input.is_none_or(|input| {
                now.saturating_duration_since(input) < SETTLES_AFTER_NO_INPUT_FOR
            })
    }

    fn advance(&mut self, now: Instant, interval: Duration) -> Option<Instant> {
        if self.is_resting() && !self.wants_motion(now) {
            return None;
        }
        self.last_input.get_or_insert(now);

        let step = self.last_frame.map_or(Duration::ZERO, |last_frame| {
            now.saturating_duration_since(last_frame).min(LONGEST_STEP)
        });
        self.last_frame = Some(now);
        self.pace = if self.wants_motion(now) {
            (self.pace + step.as_secs_f32() / WAKING_TIME.as_secs_f32()).min(1.0)
        } else {
            (self.pace - step.as_secs_f32() / SETTLING_TIME.as_secs_f32()).max(0.0)
        };
        self.shown += step.mul_f32(self.pace);

        if self.pace == 0.0 && !self.wants_motion(now) {
            self.last_frame = None;
            self.next_frame = None;
            return None;
        }

        let due = self.next_frame.unwrap_or(now);
        if now < due {
            return Some(due);
        }
        let following = due + interval;
        let next_frame = if following > now {
            following
        } else {
            now + interval
        };
        self.next_frame = Some(next_frame);
        Some(next_frame)
    }

    fn notice_input(&mut self, now: Instant) -> bool {
        self.last_input = Some(now);
        self.is_resting() && !self.is_unfocused
    }
}

impl shader::Program<Message> for SceneBackdrop {
    type State = Clock;
    type Primitive = SceneFrame;

    fn update(
        &self,
        clock: &mut Clock,
        event: &Event,
        _bounds: Rectangle,
        _cursor: Cursor,
    ) -> Option<shader::Action<Message>> {
        let wakes = match event {
            Event::Window(window::Event::RedrawRequested(now)) => {
                return clock
                    .advance(*now, self.scene.frame_interval())
                    .map(shader::Action::request_redraw_at);
            }
            Event::Window(window::Event::Unfocused) => {
                clock.is_unfocused = true;
                false
            }
            Event::Window(window::Event::Focused) => {
                clock.is_unfocused = false;
                clock.notice_input(Instant::now())
            }
            Event::Mouse(_) | Event::Keyboard(_) | Event::Touch(_) => {
                clock.notice_input(Instant::now())
            }
            _ => false,
        };
        wakes.then(shader::Action::request_redraw)
    }

    fn draw(&self, clock: &Clock, _cursor: Cursor, _bounds: Rectangle) -> SceneFrame {
        SceneFrame {
            scene: self.scene,
            seconds: clock.seconds(),
        }
    }
}

#[derive(Debug)]
struct SceneFrame {
    scene: Scene,
    seconds: f32,
}

impl shader::Primitive for SceneFrame {
    type Pipeline = ScenePipeline;

    fn prepare(
        &self,
        pipeline: &mut ScenePipeline,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bounds: &Rectangle,
        viewport: &Viewport,
    ) {
        if !pipeline.prepare_painter(device, self.scene) {
            return;
        }
        let presence = pipeline.presence_of(self.scene);
        let uniforms = [bounds.width, bounds.height, self.seconds, presence];
        queue.write_buffer(&pipeline.uniforms, 0, bytemuck::cast_slice(&uniforms));

        if let Some(pixels_per_texel) = self.scene.pixels_per_glow_texel() {
            let texels_per_point = viewport.scale_factor() / pixels_per_texel;
            pipeline.fit_glow_to(device, glow_size(bounds, texels_per_point));
            pipeline.paint_glow(device, queue, self.scene);
        }
    }

    fn draw(&self, pipeline: &ScenePipeline, render_pass: &mut wgpu::RenderPass<'_>) -> bool {
        let Some(painter) = pipeline.painter(self.scene) else {
            return true;
        };
        render_pass.set_bind_group(0, &pipeline.scene_group, &[]);
        if painter.glow.is_some() {
            let Some(glow) = &pipeline.glow else {
                return true;
            };
            render_pass.set_bind_group(1, &glow.group, &[]);
        }
        render_pass.set_pipeline(&painter.show);
        render_pass.draw(0..3, 0..1);
        if let Some(motes) = &painter.motes {
            render_pass.set_pipeline(motes);
            render_pass.draw(0..4, 0..self.scene.motes());
        }
        true
    }
}

fn glow_size(bounds: &Rectangle, texels_per_point: f32) -> (u32, u32) {
    let texels = |length: f32| ((length * texels_per_point).ceil() as u32).max(1);
    (texels(bounds.width), texels(bounds.height))
}

struct ScenePipeline {
    format: wgpu::TextureFormat,
    uniforms: wgpu::Buffer,
    scene_group: wgpu::BindGroup,
    glow_layout: wgpu::BindGroupLayout,
    layouts: Layouts,
    sampler: wgpu::Sampler,
    glow: Option<GlowTexture>,
    painters: [PainterSlot; Scene::COUNT],
}

#[derive(Default)]
enum PainterSlot {
    #[default]
    Unstarted,
    Compiling(thread::JoinHandle<Painter>),
    Ready(Painter, Instant),
    Failed,
}

#[derive(Clone)]
struct Layouts {
    scene_only: wgpu::PipelineLayout,
    with_glow: wgpu::PipelineLayout,
}

struct GlowTexture {
    size: (u32, u32),
    view: wgpu::TextureView,
    group: wgpu::BindGroup,
}

struct Painter {
    glow: Option<wgpu::RenderPipeline>,
    show: wgpu::RenderPipeline,
    motes: Option<wgpu::RenderPipeline>,
}

impl ScenePipeline {
    fn prepare_painter(&mut self, device: &wgpu::Device, scene: Scene) -> bool {
        let slot = &mut self.painters[scene as usize];
        *slot = match std::mem::take(slot) {
            PainterSlot::Unstarted => {
                let (device, format, layouts) = (device.clone(), self.format, self.layouts.clone());
                PainterSlot::Compiling(thread::spawn(move || {
                    Painter::new(&device, scene, format, &layouts)
                }))
            }
            PainterSlot::Compiling(compiling) if compiling.is_finished() => {
                match compiling.join() {
                    Ok(painter) => PainterSlot::Ready(painter, Instant::now()),
                    Err(_) => PainterSlot::Failed,
                }
            }
            unchanged => unchanged,
        };
        matches!(slot, PainterSlot::Ready(..))
    }

    fn painter(&self, scene: Scene) -> Option<&Painter> {
        match &self.painters[scene as usize] {
            PainterSlot::Ready(painter, _) => Some(painter),
            PainterSlot::Unstarted | PainterSlot::Compiling(_) | PainterSlot::Failed => None,
        }
    }

    fn presence_of(&self, scene: Scene) -> f32 {
        match &self.painters[scene as usize] {
            PainterSlot::Ready(_, ready_at) => {
                (ready_at.elapsed().as_secs_f32() / FADING_IN_TIME.as_secs_f32()).min(1.0)
            }
            PainterSlot::Unstarted | PainterSlot::Compiling(_) | PainterSlot::Failed => 0.0,
        }
    }

    fn fit_glow_to(&mut self, device: &wgpu::Device, size: (u32, u32)) {
        if self.glow.as_ref().is_some_and(|glow| glow.size == size) {
            return;
        }
        let view = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("scene glow"),
                size: wgpu::Extent3d {
                    width: size.0,
                    height: size.1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: GLOW_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default());
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("scene glow"),
            layout: &self.glow_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        self.glow = Some(GlowTexture { size, view, group });
    }

    fn paint_glow(&self, device: &wgpu::Device, queue: &wgpu::Queue, scene: Scene) {
        let Some(painter) = self.painter(scene) else {
            return;
        };
        let (Some(paint), Some(glow)) = (&painter.glow, &self.glow) else {
            return;
        };
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("scene glow"),
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scene glow"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &glow.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(paint);
            pass.set_bind_group(0, &self.scene_group, &[]);
            pass.draw(0..3, 0..1);
        }
        queue.submit([encoder.finish()]);
    }
}

impl shader::Pipeline for ScenePipeline {
    fn new(device: &wgpu::Device, _queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let scene_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("scene"),
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
        let glow_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("scene glow"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let layouts = Layouts {
            scene_only: device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("scene"),
                bind_group_layouts: &[&scene_layout],
                push_constant_ranges: &[],
            }),
            with_glow: device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("scene with glow"),
                bind_group_layouts: &[&scene_layout, &glow_layout],
                push_constant_ranges: &[],
            }),
        };

        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("scene"),
            size: std::mem::size_of::<[f32; 4]>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let scene_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("scene"),
            layout: &scene_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms.as_entire_binding(),
            }],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("scene glow"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..wgpu::SamplerDescriptor::default()
        });

        Self {
            format,
            uniforms,
            scene_group,
            glow_layout,
            layouts,
            sampler,
            glow: None,
            painters: Default::default(),
        }
    }
}

struct Drawing<'a> {
    label: &'a str,
    layout: &'a wgpu::PipelineLayout,
    vertex: &'a str,
    fragment: &'a str,
    target: wgpu::ColorTargetState,
    topology: wgpu::PrimitiveTopology,
}

impl Painter {
    fn new(
        device: &wgpu::Device,
        scene: Scene,
        format: wgpu::TextureFormat,
        layouts: &Layouts,
    ) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(scene.name()),
            source: wgpu::ShaderSource::Wgsl(scene.shader_source().into()),
        });
        let on_srgb = format.is_srgb();
        let has_glow = scene.pixels_per_glow_texel().is_some();

        let glow = has_glow.then(|| {
            render_pipeline(
                device,
                &module,
                Drawing {
                    label: "scene glow",
                    layout: &layouts.scene_only,
                    vertex: "cover",
                    fragment: "paint_glow",
                    target: opaque(GLOW_FORMAT),
                    topology: wgpu::PrimitiveTopology::TriangleList,
                },
            )
        });
        let show = render_pipeline(
            device,
            &module,
            Drawing {
                label: scene.name(),
                layout: if has_glow {
                    &layouts.with_glow
                } else {
                    &layouts.scene_only
                },
                vertex: "cover",
                fragment: if on_srgb { "show_on_srgb" } else { "show" },
                target: opaque(format),
                topology: wgpu::PrimitiveTopology::TriangleList,
            },
        );
        let motes = (scene.motes() > 0).then(|| {
            render_pipeline(
                device,
                &module,
                Drawing {
                    label: "scene motes",
                    layout: &layouts.scene_only,
                    vertex: "place_mote",
                    fragment: if on_srgb { "shine_on_srgb" } else { "shine" },
                    target: adding_light(format),
                    topology: wgpu::PrimitiveTopology::TriangleStrip,
                },
            )
        });

        Self { glow, show, motes }
    }
}

fn opaque(format: wgpu::TextureFormat) -> wgpu::ColorTargetState {
    wgpu::ColorTargetState {
        format,
        blend: None,
        write_mask: wgpu::ColorWrites::ALL,
    }
}

fn adding_light(format: wgpu::TextureFormat) -> wgpu::ColorTargetState {
    wgpu::ColorTargetState {
        format,
        blend: Some(wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::Zero,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
        }),
        write_mask: wgpu::ColorWrites::ALL,
    }
}

fn render_pipeline(
    device: &wgpu::Device,
    module: &wgpu::ShaderModule,
    drawing: Drawing<'_>,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(drawing.label),
        layout: Some(drawing.layout),
        vertex: wgpu::VertexState {
            module,
            entry_point: Some(drawing.vertex),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState {
            topology: drawing.topology,
            ..wgpu::PrimitiveState::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module,
            entry_point: Some(drawing.fragment),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(drawing.target)],
        }),
        multiview: None,
        cache: None,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        Clock, Duration, Instant, LONGEST_STEP, Rectangle, SETTLES_AFTER_NO_INPUT_FOR,
        SETTLING_TIME, Scene, glow_size,
    };

    const INTERVAL: Duration = Duration::from_millis(50);

    fn run_until_settled(clock: &mut Clock, start: Instant) -> Instant {
        let mut now = start;
        while let Some(frame) = clock.advance(now, INTERVAL) {
            assert!(frame > now);
            now = frame;
            assert!(now < start + Duration::from_secs(600));
        }
        now
    }

    #[test]
    fn schedules_frames_at_the_scene_rate() {
        let start = Instant::now();
        let mut clock = Clock::default();
        assert_eq!(clock.advance(start, INTERVAL), Some(start + INTERVAL));
        assert_eq!(
            clock.advance(start + INTERVAL, INTERVAL),
            Some(start + INTERVAL * 2)
        );
    }

    #[test]
    fn redraws_between_frames_keep_the_schedule() {
        let start = Instant::now();
        let mut clock = Clock::default();
        clock.advance(start, INTERVAL);
        let early = start + INTERVAL / 3;
        assert_eq!(clock.advance(early, INTERVAL), Some(start + INTERVAL));
    }

    #[test]
    fn settles_after_a_minute_without_input() {
        let start = Instant::now();
        let mut clock = Clock::default();
        let settled_at = run_until_settled(&mut clock, start);
        assert!(settled_at >= start + SETTLES_AFTER_NO_INPUT_FOR);
        assert!(settled_at <= start + SETTLES_AFTER_NO_INPUT_FOR + SETTLING_TIME + INTERVAL * 2);
        assert_eq!(clock.advance(settled_at + INTERVAL, INTERVAL), None);
    }

    #[test]
    fn input_wakes_a_settled_scene() {
        let start = Instant::now();
        let mut clock = Clock::default();
        let settled_at = run_until_settled(&mut clock, start);
        let moved_at = settled_at + Duration::from_secs(5);
        assert!(clock.notice_input(moved_at));
        assert!(clock.advance(moved_at, INTERVAL).is_some());
    }

    #[test]
    fn losing_focus_settles_the_scene_until_it_comes_back() {
        let start = Instant::now();
        let mut clock = Clock::default();
        clock.advance(start, INTERVAL);
        clock.is_unfocused = true;
        let settled_at = run_until_settled(&mut clock, start + INTERVAL);
        assert!(settled_at <= start + SETTLING_TIME + INTERVAL * 3);
        assert!(!clock.notice_input(settled_at));
        clock.is_unfocused = false;
        assert!(clock.notice_input(settled_at));
    }

    #[test]
    fn a_long_gap_moves_the_scene_by_at_most_one_step() {
        let start = Instant::now();
        let mut clock = Clock::default();
        clock.advance(start, INTERVAL);
        clock.advance(start + Duration::from_secs(30), INTERVAL);
        assert!(clock.seconds() <= LONGEST_STEP.as_secs_f32());
    }

    #[test]
    fn glow_texture_rounds_up_and_never_vanishes() {
        let window = Rectangle::new(iced::Point::ORIGIN, iced::Size::new(801.0, 600.0));
        assert_eq!(glow_size(&window, 0.25), (201, 150));
        assert_eq!(glow_size(&Rectangle::default(), 0.25), (1, 1));
    }

    #[test]
    fn every_scene_shader_is_valid_wgsl() {
        for scene in Scene::ALL {
            let source = scene.shader_source();
            let module = naga::front::wgsl::parse_str(&source).unwrap_or_else(|error| {
                panic!("{}: {}", scene.name(), error.emit_to_string(&source))
            });
            naga::valid::Validator::new(
                naga::valid::ValidationFlags::all(),
                naga::valid::Capabilities::default(),
            )
            .validate(&module)
            .unwrap_or_else(|error| panic!("{}: {error:?}", scene.name()));
        }
    }
}
