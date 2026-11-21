use crate::components::imported_theme::brightness;
use crate::quadra::Message;
use crate::state::{Animations, BackgroundAnimation};
use iced::mouse::Cursor;
use iced::time::{Duration, Instant};
use iced::wgpu;
use iced::widget::shader::{self, Viewport};
use iced::{Color, Element, Event, Length, Rectangle, color, window};
use std::sync::atomic::{AtomicU32, Ordering};
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
    Embers,
    Leaves,
    Scanlines,
    Waves,
    Fireflies,
    Bokeh,
    Warp,
    Rain,
    Lava,
    Silk,
    Snow,
    Petals,
    Horizon,
    Lines,
    Facets,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    base: Color,
    lights: [Color; 3],
}

impl Scene {
    const ALL: [Scene; 16] = [
        Self::Aurora,
        Self::Embers,
        Self::Leaves,
        Self::Scanlines,
        Self::Waves,
        Self::Fireflies,
        Self::Bokeh,
        Self::Warp,
        Self::Rain,
        Self::Lava,
        Self::Silk,
        Self::Snow,
        Self::Petals,
        Self::Horizon,
        Self::Lines,
        Self::Facets,
    ];
    const COUNT: usize = Self::ALL.len();

    pub fn chosen(animation: BackgroundAnimation) -> Option<Self> {
        match animation {
            BackgroundAnimation::ThemeDefault | BackgroundAnimation::None => None,
            BackgroundAnimation::Aurora => Some(Self::Aurora),
            BackgroundAnimation::Embers => Some(Self::Embers),
            BackgroundAnimation::Leaves => Some(Self::Leaves),
            BackgroundAnimation::Scanlines => Some(Self::Scanlines),
            BackgroundAnimation::Waves => Some(Self::Waves),
            BackgroundAnimation::Fireflies => Some(Self::Fireflies),
            BackgroundAnimation::Bokeh => Some(Self::Bokeh),
            BackgroundAnimation::Warp => Some(Self::Warp),
            BackgroundAnimation::Rain => Some(Self::Rain),
            BackgroundAnimation::Lava => Some(Self::Lava),
            BackgroundAnimation::Silk => Some(Self::Silk),
            BackgroundAnimation::Snow => Some(Self::Snow),
            BackgroundAnimation::Petals => Some(Self::Petals),
            BackgroundAnimation::Horizon => Some(Self::Horizon),
            BackgroundAnimation::Lines => Some(Self::Lines),
            BackgroundAnimation::Facets => Some(Self::Facets),
        }
    }

    pub fn view(self, palette: Palette, motion: Animations) -> Element<'static, Message> {
        self.played(palette, motion, Playing::Backdrop)
    }

    pub fn preview(self, palette: Palette, motion: Animations) -> Element<'static, Message> {
        self.played(palette, motion, Playing::Preview)
    }

    fn played(
        self,
        palette: Palette,
        motion: Animations,
        playing: Playing,
    ) -> Element<'static, Message> {
        shader::Shader::new(SceneBackdrop {
            scene: self,
            palette,
            motion,
            playing,
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    pub fn default_lights(self) -> &'static [Color] {
        const AURORA: [Color; 3] = [color!(0xff5fa2), color!(0x8f66ff), color!(0xf2e6ff)];
        const EMBERS: [Color; 2] = [color!(0xff661c), color!(0xffd180)];
        const LEAVES: [Color; 2] = [color!(0xfffbf1), color!(0x3c4a2c)];
        const SCANLINES: [Color; 1] = [color!(0x39ff7a)];
        const WAVES: [Color; 3] = [color!(0x2f6fd0), color!(0x19b5e6), color!(0x8be3f5)];
        const FIREFLIES: [Color; 2] = [color!(0xc8ff6a), color!(0xffd36a)];
        const BOKEH: [Color; 3] = [color!(0xff6fb5), color!(0x6fb7ff), color!(0xffd56f)];
        const WARP: [Color; 3] = [color!(0x5b4dff), color!(0xd6e6ff), color!(0xff9de2)];
        const RAIN: [Color; 2] = [color!(0x9ec9ff), color!(0xd2e2ff)];
        const LAVA: [Color; 3] = [color!(0xff4f6d), color!(0xffb347), color!(0x6a2cff)];
        const SILK: [Color; 3] = [color!(0x3a2cff), color!(0xc23bff), color!(0xffa6e8)];
        const SNOW: [Color; 2] = [color!(0xffffff), color!(0xbcd8ff)];
        const PETALS: [Color; 3] = [color!(0xffc2db), color!(0xff9cc4), color!(0xffe3ee)];
        const HORIZON: [Color; 3] = [color!(0xff3df2), color!(0xffb03a), color!(0x7a2cff)];
        const LINES: [Color; 2] = [color!(0x8c8c8c), color!(0xffffff)];
        const FACETS: [Color; 3] = [color!(0x4b52d6), color!(0x38d0ff), color!(0xa98cff)];
        match self {
            Self::Aurora => &AURORA,
            Self::Embers => &EMBERS,
            Self::Leaves => &LEAVES,
            Self::Scanlines => &SCANLINES,
            Self::Waves => &WAVES,
            Self::Fireflies => &FIREFLIES,
            Self::Bokeh => &BOKEH,
            Self::Warp => &WARP,
            Self::Rain => &RAIN,
            Self::Lava => &LAVA,
            Self::Silk => &SILK,
            Self::Snow => &SNOW,
            Self::Petals => &PETALS,
            Self::Horizon => &HORIZON,
            Self::Lines => &LINES,
            Self::Facets => &FACETS,
        }
    }

    pub fn light_names(self) -> &'static [&'static str] {
        match self {
            Self::Aurora => &["Lower curtain", "Upper curtain", "Stars"],
            Self::Embers => &["Flame", "Sparks"],
            Self::Leaves => &["Sunlight", "Shadow"],
            Self::Scanlines => &["Glow"],
            Self::Waves => &["Back wave", "Middle wave", "Front wave"],
            Self::Fireflies => &["Glow", "Second glow"],
            Self::Bokeh => &["First light", "Second light", "Third light"],
            Self::Warp => &["Nebula", "Stars", "Rare stars"],
            Self::Rain => &["Rain", "Lightning"],
            Self::Lava => &["Cool wax", "Hot wax", "Lamp glass"],
            Self::Silk => &["Deep fold", "Middle fold", "Highlight"],
            Self::Snow => &["Snowflakes", "Snowdrift"],
            Self::Petals => &["Haze", "First petals", "Second petals"],
            Self::Horizon => &["Grid", "Sun", "Haze"],
            Self::Lines => &["Lines", "Highlight"],
            Self::Facets => &["Facets", "Sweep", "Edges"],
        }
    }

    pub fn palette(self, base: Color, chosen: &[Option<Color>]) -> Palette {
        let mut lights = [Color::WHITE; 3];
        let defaults = self.default_lights();
        for (index, light) in lights.iter_mut().enumerate() {
            let fallback = defaults
                .get(index)
                .or(defaults.last())
                .copied()
                .unwrap_or(Color::WHITE);
            *light = chosen.get(index).copied().flatten().unwrap_or(fallback);
        }
        Palette {
            base: Color { a: 1.0, ..base },
            lights,
        }
    }

    fn frames_per_second(self) -> u32 {
        match self {
            Self::Leaves | Self::Bokeh | Self::Silk | Self::Lava | Self::Facets => 15,
            Self::Aurora
            | Self::Embers
            | Self::Scanlines
            | Self::Waves
            | Self::Fireflies
            | Self::Snow
            | Self::Petals
            | Self::Lines => 20,
            Self::Warp | Self::Rain | Self::Horizon => 30,
        }
    }

    fn pixels_per_glow_texel(self) -> Option<f32> {
        match self {
            Self::Aurora | Self::Waves => Some(4.0),
            Self::Embers => Some(6.0),
            Self::Lava | Self::Silk => Some(6.0),
            Self::Leaves
            | Self::Fireflies
            | Self::Bokeh
            | Self::Warp
            | Self::Rain
            | Self::Snow
            | Self::Petals => Some(8.0),
            Self::Scanlines | Self::Horizon | Self::Lines | Self::Facets => None,
        }
    }

    fn motes(self) -> u32 {
        match self {
            Self::Aurora => 70,
            Self::Embers => 72,
            Self::Fireflies => 40,
            Self::Bokeh => 26,
            Self::Warp => 120,
            Self::Rain => 110,
            Self::Snow => 140,
            Self::Petals => 46,
            Self::Leaves
            | Self::Scanlines
            | Self::Waves
            | Self::Lava
            | Self::Silk
            | Self::Horizon
            | Self::Lines
            | Self::Facets => 0,
        }
    }

    fn mote_shape(self) -> (&'static str, &'static str) {
        match self {
            Self::Bokeh => ("shine_disc", "shine_disc_on_srgb"),
            _ => ("shine", "shine_on_srgb"),
        }
    }

    fn mote_shade(self) -> (&'static str, &'static str) {
        match self {
            Self::Bokeh => ("shade_disc", "shade_disc_on_srgb"),
            _ => ("shade", "shade_on_srgb"),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Aurora => "aurora",
            Self::Embers => "embers",
            Self::Leaves => "leaves",
            Self::Scanlines => "scanlines",
            Self::Waves => "waves",
            Self::Fireflies => "fireflies",
            Self::Bokeh => "bokeh",
            Self::Warp => "warp",
            Self::Rain => "rain",
            Self::Lava => "lava",
            Self::Silk => "silk",
            Self::Snow => "snow",
            Self::Petals => "petals",
            Self::Horizon => "horizon",
            Self::Lines => "lines",
            Self::Facets => "facets",
        }
    }

    fn shader_source(self) -> String {
        let own = match self {
            Self::Aurora => include_str!("scenes/aurora.wgsl"),
            Self::Embers => include_str!("scenes/embers.wgsl"),
            Self::Leaves => include_str!("scenes/leaves.wgsl"),
            Self::Scanlines => include_str!("scenes/scanlines.wgsl"),
            Self::Waves => include_str!("scenes/waves.wgsl"),
            Self::Fireflies => include_str!("scenes/fireflies.wgsl"),
            Self::Bokeh => include_str!("scenes/bokeh.wgsl"),
            Self::Warp => include_str!("scenes/warp.wgsl"),
            Self::Rain => include_str!("scenes/rain.wgsl"),
            Self::Lava => include_str!("scenes/lava.wgsl"),
            Self::Silk => include_str!("scenes/silk.wgsl"),
            Self::Snow => include_str!("scenes/snow.wgsl"),
            Self::Petals => include_str!("scenes/petals.wgsl"),
            Self::Horizon => include_str!("scenes/horizon.wgsl"),
            Self::Lines => include_str!("scenes/lines.wgsl"),
            Self::Facets => include_str!("scenes/facets.wgsl"),
        };
        let glow_pass = if self.pixels_per_glow_texel().is_some() {
            GLOW_PASS
        } else {
            ""
        };
        format!("{PRELUDE}\n{glow_pass}\n{own}")
    }
}

static FULLY_SHOWN_SCENES: AtomicU32 = AtomicU32::new(0);

fn is_fully_shown(scene: Scene) -> bool {
    FULLY_SHOWN_SCENES.load(Ordering::Relaxed) & (1 << scene as u32) != 0
}

fn mark_fully_shown(scene: Scene) {
    FULLY_SHOWN_SCENES.fetch_or(1 << scene as u32, Ordering::Relaxed);
}

const WARMING_UP: Duration = Duration::from_millis(2500);
const PREVIEW_STARTS_AT: Duration = Duration::from_secs(9);
const REDUCED_PACE: f32 = 0.35;
const REDUCED_FRAMES_PER_SECOND: u32 = 10;
const REDUCED_SETTLES_AFTER_NO_INPUT_FOR: Duration = Duration::from_secs(15);
const UNUSED_SLOTS_LAST: Duration = Duration::from_secs(3);
const LIGHT_WINDOW_BRIGHTNESS: f32 = 0.55;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Playing {
    Backdrop,
    Preview,
}

struct SceneBackdrop {
    scene: Scene,
    palette: Palette,
    motion: Animations,
    playing: Playing,
}

impl SceneBackdrop {
    fn rules(&self) -> Rules {
        let reduced = self.motion != Animations::Full;
        let frames_per_second = if reduced {
            REDUCED_FRAMES_PER_SECOND
        } else {
            self.scene.frames_per_second()
        };
        Rules {
            interval: Duration::from_secs(1) / frames_per_second,
            top_pace: match self.motion {
                Animations::Full => 1.0,
                Animations::Reduced => REDUCED_PACE,
                Animations::Off => 0.0,
            },
            settles_after: if reduced {
                REDUCED_SETTLES_AFTER_NO_INPUT_FOR
            } else {
                SETTLES_AFTER_NO_INPUT_FOR
            },
            playing: self.playing,
            needs_warming: !is_fully_shown(self.scene),
        }
    }
}

#[derive(Clone, Copy)]
struct Rules {
    interval: Duration,
    top_pace: f32,
    settles_after: Duration,
    playing: Playing,
    needs_warming: bool,
}

#[derive(Default)]
struct Clock {
    shown: Duration,
    pace: f32,
    last_frame: Option<Instant>,
    next_frame: Option<Instant>,
    last_input: Option<Instant>,
    warm_until: Option<Instant>,
    is_unfocused: bool,
    is_hovered: bool,
}

impl Clock {
    fn seconds(&self) -> f32 {
        (self.shown.as_secs_f64() % TIMELINE_LENGTH.as_secs_f64()) as f32
    }

    fn is_resting(&self) -> bool {
        self.last_frame.is_none()
    }

    fn wants_motion(&self, now: Instant, rules: Rules) -> bool {
        if self.is_unfocused || rules.top_pace == 0.0 {
            return false;
        }
        match rules.playing {
            Playing::Preview => self.is_hovered,
            Playing::Backdrop => self
                .last_input
                .is_none_or(|input| now.saturating_duration_since(input) < rules.settles_after),
        }
    }

    fn advance(&mut self, now: Instant, rules: Rules) -> Option<Instant> {
        if self.warm_until.is_none() && rules.playing == Playing::Preview {
            self.shown = PREVIEW_STARTS_AT;
        }
        let warm_until = *self.warm_until.get_or_insert(now + WARMING_UP);
        let is_warming_up = rules.needs_warming && warm_until > now;
        let wants_motion = self.wants_motion(now, rules);
        if self.is_resting() && !wants_motion && !is_warming_up {
            return None;
        }
        self.last_input.get_or_insert(now);

        let step = self.last_frame.map_or(Duration::ZERO, |last_frame| {
            now.saturating_duration_since(last_frame).min(LONGEST_STEP)
        });
        self.last_frame = Some(now);
        let target = if wants_motion { rules.top_pace } else { 0.0 };
        self.pace = if self.pace < target {
            (self.pace + step.as_secs_f32() / WAKING_TIME.as_secs_f32()).min(target)
        } else {
            (self.pace - step.as_secs_f32() / SETTLING_TIME.as_secs_f32()).max(target)
        };
        self.shown += step.mul_f32(self.pace);

        if self.pace == 0.0 && !wants_motion && !is_warming_up {
            self.last_frame = None;
            self.next_frame = None;
            return None;
        }

        let due = self.next_frame.unwrap_or(now);
        if now < due {
            return Some(due);
        }
        let following = due + rules.interval;
        let next_frame = if following > now {
            following
        } else {
            now + rules.interval
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
        bounds: Rectangle,
        cursor: Cursor,
    ) -> Option<shader::Action<Message>> {
        let wakes = match event {
            Event::Window(window::Event::RedrawRequested(now)) => {
                return clock
                    .advance(*now, self.rules())
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
                let was_hovered = clock.is_hovered;
                clock.is_hovered = cursor.is_over(bounds);
                let noticed = clock.notice_input(Instant::now());
                match self.playing {
                    Playing::Backdrop => noticed,
                    Playing::Preview => clock.is_hovered && !was_hovered && clock.is_resting(),
                }
            }
            _ => false,
        };
        wakes.then(shader::Action::request_redraw)
    }

    fn draw(&self, clock: &Clock, _cursor: Cursor, bounds: Rectangle) -> SceneFrame {
        SceneFrame {
            scene: self.scene,
            palette: self.palette,
            seconds: clock.seconds(),
            place: [bounds.x, bounds.y, bounds.width, bounds.height]
                .map(|value| value.round() as i32),
        }
    }
}

#[derive(Debug)]
struct SceneFrame {
    scene: Scene,
    palette: Palette,
    seconds: f32,
    place: [i32; 4],
}

const UNIFORM_FLOATS: usize = 36;

impl SceneFrame {
    fn uniforms(&self, bounds: &Rectangle, presence: f32) -> [f32; UNIFORM_FLOATS] {
        let mut uniforms = [0.0; UNIFORM_FLOATS];
        uniforms[..4].copy_from_slice(&[bounds.width, bounds.height, self.seconds, presence]);
        let colors = std::iter::once(self.palette.base).chain(self.palette.lights);
        for (index, color) in colors.enumerate() {
            let srgb = [color.r, color.g, color.b, color.a];
            uniforms[4 + index * 4..8 + index * 4].copy_from_slice(&srgb);
            uniforms[20 + index * 4..24 + index * 4].copy_from_slice(&color.into_linear());
        }
        uniforms
    }

    fn is_on_light_window(&self) -> bool {
        brightness(self.palette.base) > LIGHT_WINDOW_BRIGHTNESS
    }
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
        let slot = pipeline.slot_for(device, self.place);
        let uniforms = self.uniforms(bounds, presence);
        queue.write_buffer(
            &pipeline.slots[slot].uniforms,
            0,
            bytemuck::cast_slice(&uniforms),
        );

        if let Some(pixels_per_texel) = self.scene.pixels_per_glow_texel() {
            let texels_per_point = viewport.scale_factor() / pixels_per_texel;
            pipeline.fit_glow_to(device, slot, glow_size(bounds, texels_per_point));
            pipeline.paint_glow(device, queue, self.scene, slot);
        }
    }

    fn draw(&self, pipeline: &ScenePipeline, render_pass: &mut wgpu::RenderPass<'_>) -> bool {
        let (Some(painter), Some(slot)) = (
            pipeline.painter(self.scene),
            pipeline.slots.iter().find(|slot| slot.place == self.place),
        ) else {
            return true;
        };
        render_pass.set_bind_group(0, &slot.scene_group, &[]);
        if painter.glow.is_some() {
            let Some(glow) = &slot.glow else {
                return true;
            };
            render_pass.set_bind_group(1, &glow.group, &[]);
        }
        render_pass.set_pipeline(&painter.show);
        render_pass.draw(0..3, 0..1);
        let motes = if self.is_on_light_window() {
            &painter.shades
        } else {
            &painter.motes
        };
        if let Some(motes) = motes {
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
    scene_layout: wgpu::BindGroupLayout,
    glow_layout: wgpu::BindGroupLayout,
    layouts: Layouts,
    sampler: wgpu::Sampler,
    slots: Vec<Slot>,
    painters: [PainterSlot; Scene::COUNT],
}

struct Slot {
    place: [i32; 4],
    uniforms: wgpu::Buffer,
    scene_group: wgpu::BindGroup,
    glow: Option<GlowTexture>,
    used_at: Instant,
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
    shades: Option<wgpu::RenderPipeline>,
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
                let presence =
                    (ready_at.elapsed().as_secs_f32() / FADING_IN_TIME.as_secs_f32()).min(1.0);
                if presence == 1.0 {
                    mark_fully_shown(scene);
                }
                presence
            }
            PainterSlot::Unstarted | PainterSlot::Compiling(_) | PainterSlot::Failed => 0.0,
        }
    }

    fn slot_for(&mut self, device: &wgpu::Device, place: [i32; 4]) -> usize {
        let now = Instant::now();
        self.slots.retain(|slot| {
            slot.place == place || now.duration_since(slot.used_at) < UNUSED_SLOTS_LAST
        });
        if let Some(index) = self.slots.iter().position(|slot| slot.place == place) {
            self.slots[index].used_at = now;
            return index;
        }
        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("scene"),
            size: std::mem::size_of::<[f32; UNIFORM_FLOATS]>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let scene_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("scene"),
            layout: &self.scene_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms.as_entire_binding(),
            }],
        });
        self.slots.push(Slot {
            place,
            uniforms,
            scene_group,
            glow: None,
            used_at: now,
        });
        self.slots.len() - 1
    }

    fn fit_glow_to(&mut self, device: &wgpu::Device, slot: usize, size: (u32, u32)) {
        if self.slots[slot]
            .glow
            .as_ref()
            .is_some_and(|glow| glow.size == size)
        {
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
        self.slots[slot].glow = Some(GlowTexture { size, view, group });
    }

    fn paint_glow(&self, device: &wgpu::Device, queue: &wgpu::Queue, scene: Scene, slot: usize) {
        let Some(painter) = self.painter(scene) else {
            return;
        };
        let slot = &self.slots[slot];
        let (Some(paint), Some(glow)) = (&painter.glow, &slot.glow) else {
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
            pass.set_bind_group(0, &slot.scene_group, &[]);
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
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("scene glow"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..wgpu::SamplerDescriptor::default()
        });

        Self {
            format,
            scene_layout,
            glow_layout,
            layouts,
            sampler,
            slots: Vec::new(),
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
        let mote_pipeline = |(fragment, fragment_on_srgb): (&str, &str), target| {
            (scene.motes() > 0).then(|| {
                render_pipeline(
                    device,
                    &module,
                    Drawing {
                        label: "scene motes",
                        layout: &layouts.scene_only,
                        vertex: "place_mote",
                        fragment: if on_srgb { fragment_on_srgb } else { fragment },
                        target,
                        topology: wgpu::PrimitiveTopology::TriangleStrip,
                    },
                )
            })
        };
        let motes = mote_pipeline(scene.mote_shape(), adding_light(format));
        let shades = mote_pipeline(scene.mote_shade(), laying_ink(format));

        Self {
            glow,
            show,
            motes,
            shades,
        }
    }
}

fn opaque(format: wgpu::TextureFormat) -> wgpu::ColorTargetState {
    wgpu::ColorTargetState {
        format,
        blend: None,
        write_mask: wgpu::ColorWrites::ALL,
    }
}

fn laying_ink(format: wgpu::TextureFormat) -> wgpu::ColorTargetState {
    wgpu::ColorTargetState {
        format,
        blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
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
        Clock, Duration, Instant, LONGEST_STEP, Playing, REDUCED_PACE, Rectangle, Rules,
        SETTLES_AFTER_NO_INPUT_FOR, SETTLING_TIME, Scene, glow_size,
    };

    const INTERVAL: Duration = Duration::from_millis(50);

    fn backdrop() -> Rules {
        Rules {
            interval: INTERVAL,
            top_pace: 1.0,
            settles_after: SETTLES_AFTER_NO_INPUT_FOR,
            playing: Playing::Backdrop,
            needs_warming: true,
        }
    }

    #[test]
    fn previews_of_ready_scenes_ask_for_no_frames() {
        let mut clock = Clock::default();
        let preview = Rules {
            playing: Playing::Preview,
            needs_warming: false,
            ..backdrop()
        };
        assert_eq!(clock.advance(Instant::now(), preview), None);
    }

    fn run_until_settled(clock: &mut Clock, start: Instant) -> Instant {
        let mut now = start;
        while let Some(frame) = clock.advance(now, backdrop()) {
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
        assert_eq!(clock.advance(start, backdrop()), Some(start + INTERVAL));
        assert_eq!(
            clock.advance(start + INTERVAL, backdrop()),
            Some(start + INTERVAL * 2)
        );
    }

    #[test]
    fn redraws_between_frames_keep_the_schedule() {
        let start = Instant::now();
        let mut clock = Clock::default();
        clock.advance(start, backdrop());
        let early = start + INTERVAL / 3;
        assert_eq!(clock.advance(early, backdrop()), Some(start + INTERVAL));
    }

    #[test]
    fn settles_after_a_minute_without_input() {
        let start = Instant::now();
        let mut clock = Clock::default();
        let settled_at = run_until_settled(&mut clock, start);
        assert!(settled_at >= start + SETTLES_AFTER_NO_INPUT_FOR);
        assert!(settled_at <= start + SETTLES_AFTER_NO_INPUT_FOR + SETTLING_TIME + INTERVAL * 2);
        assert_eq!(clock.advance(settled_at + INTERVAL, backdrop()), None);
    }

    #[test]
    fn input_wakes_a_settled_scene() {
        let start = Instant::now();
        let mut clock = Clock::default();
        let settled_at = run_until_settled(&mut clock, start);
        let moved_at = settled_at + Duration::from_secs(5);
        assert!(clock.notice_input(moved_at));
        assert!(clock.advance(moved_at, backdrop()).is_some());
    }

    #[test]
    fn losing_focus_settles_the_scene_until_it_comes_back() {
        let start = Instant::now();
        let mut clock = Clock::default();
        clock.advance(start, backdrop());
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
        clock.advance(start, backdrop());
        clock.advance(start + Duration::from_secs(30), backdrop());
        assert!(clock.seconds() <= LONGEST_STEP.as_secs_f32());
    }

    #[test]
    fn previews_only_move_while_hovered() {
        let start = Instant::now();
        let mut clock = Clock::default();
        let preview = Rules {
            playing: Playing::Preview,
            ..backdrop()
        };
        let mut now = start;
        while let Some(frame) = clock.advance(now, preview) {
            now = frame;
        }
        let still = clock.seconds();
        assert!(now < start + Duration::from_secs(4));
        clock.is_hovered = true;
        let hovered_at = now + Duration::from_secs(1);
        let mut frame = clock.advance(hovered_at, preview);
        for _ in 0..40 {
            frame = frame.and_then(|frame| clock.advance(frame, preview));
        }
        assert!(clock.seconds() > still);
    }

    #[test]
    fn reduced_motion_runs_slower() {
        let start = Instant::now();
        let mut clock = Clock::default();
        let reduced = Rules {
            top_pace: REDUCED_PACE,
            ..backdrop()
        };
        let mut now = start;
        for _ in 0..100 {
            now = clock.advance(now, reduced).unwrap();
        }
        assert!(clock.pace <= REDUCED_PACE);
        assert!(clock.seconds() < (now - start).as_secs_f32() * REDUCED_PACE + 0.01);
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
