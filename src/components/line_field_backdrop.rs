use crate::quadra::Message;
use iced::mouse::Cursor;
use iced::widget::canvas::{self, Cache, Geometry};
use iced::widget::image;
use iced::{Element, Length, Point, Rectangle, Renderer, Size, Theme};
use std::f64::consts::TAU;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const WIDTH: usize = 1280;
const HEIGHT: usize = 800;
const WANDERERS: usize = 520;
const SHORTEST_LIFE: i32 = 60;
const LONGEST_LIFE: i32 = 260;
const STEP_LENGTH: f64 = 1.15;
pub const TICK_INTERVAL: Duration = Duration::from_millis(40);
const TICKS_UNTIL_DRAWN: u32 = 190;
const STEPS_PER_TICK: u32 = 3;
const BRIGHTNESS_PER_VISIT: u8 = 15;
const BRIGHTEST_LINE: u8 = 112;
const FLOW_CELL_SIZE: f64 = 150.0;
const FLOW_TURNS: f64 = 2.0;

struct Random(u64);

impl Random {
    fn seeded_by_clock() -> Self {
        let nanoseconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_nanos() as u64);
        Self(nanoseconds ^ u64::from(std::process::id()) << 32)
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

pub struct LineField {
    pixels: Vec<u8>,
    flow: FlowField,
    wanderers: Vec<Wanderer>,
    random: Random,
    ticks: u32,
    shown: Option<image::Handle>,
    uploading: Option<image::Handle>,
    drawing: Cache,
}

impl Default for LineField {
    fn default() -> Self {
        let mut random = Random::seeded_by_clock();
        Self {
            pixels: vec![0; WIDTH * HEIGHT],
            flow: FlowField::random(&mut random),
            wanderers: (0..WANDERERS)
                .map(|_| Wanderer::spawn(&mut random))
                .collect(),
            random,
            ticks: 0,
            shown: None,
            uploading: None,
            drawing: Cache::default(),
        }
    }
}

impl LineField {
    fn is_drawn(&self) -> bool {
        self.ticks >= TICKS_UNTIL_DRAWN
    }

    pub fn is_settled(&self) -> bool {
        self.is_drawn() && self.uploading.is_none()
    }

    pub fn tick(&mut self) {
        if self.is_settled() {
            return;
        }
        if let Some(uploaded) = self.uploading.take() {
            self.shown = Some(uploaded);
        }
        if !self.is_drawn() {
            self.advance();
            self.uploading = Some(self.image());
            if self.is_drawn() {
                self.pixels = Vec::new();
                self.wanderers = Vec::new();
            }
        }
        self.drawing.clear();
    }

    pub fn view(&self) -> Option<Element<'_, Message>> {
        self.shown.as_ref().or(self.uploading.as_ref())?;
        Some(
            canvas::Canvas::new(self)
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
        )
    }

    fn advance(&mut self) {
        let Self {
            pixels,
            flow,
            wanderers,
            random,
            ..
        } = self;
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

                let pixel = &mut pixels[wanderer.y as usize * WIDTH + wanderer.x as usize];
                *pixel = pixel
                    .saturating_add(BRIGHTNESS_PER_VISIT)
                    .min(BRIGHTEST_LINE);
            }
        }
        self.ticks += 1;
    }

    fn image(&self) -> image::Handle {
        let rgba = self
            .pixels
            .iter()
            .flat_map(|&brightness| [brightness, brightness, brightness, u8::MAX])
            .collect::<Vec<u8>>();
        image::Handle::from_rgba(WIDTH as u32, HEIGHT as u32, rgba)
    }
}

impl canvas::Program<Message> for LineField {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
        let drawing = self.drawing.draw(renderer, bounds.size(), |frame| {
            let covering = covering(frame.size());
            frame.with_clip(Rectangle::with_size(frame.size()), |frame| {
                for image in [&self.uploading, &self.shown].into_iter().flatten() {
                    frame.draw_image(covering, image);
                }
            });
        });
        vec![drawing]
    }
}

fn covering(area: Size) -> Rectangle {
    let scale = f32::max(area.width / WIDTH as f32, area.height / HEIGHT as f32);
    let size = Size::new(WIDTH as f32 * scale, HEIGHT as f32 * scale);
    let position = Point::new(
        (area.width - size.width) / 2.0,
        (area.height - size.height) / 2.0,
    );
    Rectangle::new(position, size)
}
