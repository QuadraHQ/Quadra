use crate::state::{AppState, Theme};
use gtk::prelude::*;
use gtk::{gdk, glib};
use std::cell::RefCell;
use std::f64::consts::TAU;
use std::rc::Rc;
use std::time::Duration;

const WIDTH: usize = 1280;
const HEIGHT: usize = 800;
const WANDERERS: usize = 520;
const SHORTEST_LIFE: i32 = 60;
const LONGEST_LIFE: i32 = 260;
const STEP_LENGTH: f64 = 1.15;
const TICK_INTERVAL: Duration = Duration::from_millis(40);
const TICKS_UNTIL_DRAWN: u32 = 190;
const STEPS_PER_TICK: u32 = 3;
const TICKS_SKIPPED_WITHOUT_ANIMATIONS: u32 = 8;
const BRIGHTNESS_PER_VISIT: u8 = 15;
const BRIGHTEST_LINE: u8 = 112;
const FLOW_CELL_SIZE: f64 = 150.0;
const FLOW_TURNS: f64 = 2.0;

struct Wanderer {
    x: f64,
    y: f64,
    steps_left: i32,
}

impl Wanderer {
    fn spawn() -> Self {
        Self {
            x: glib::random_double_range(0.0, WIDTH as f64),
            y: glib::random_double_range(0.0, HEIGHT as f64),
            steps_left: glib::random_int_range(SHORTEST_LIFE, LONGEST_LIFE + 1),
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
    fn random() -> Self {
        let columns = (WIDTH as f64 / FLOW_CELL_SIZE) as usize + 3;
        let rows = (HEIGHT as f64 / FLOW_CELL_SIZE) as usize + 3;
        let lattice = (0..columns * rows).map(|_| glib::random_double()).collect();
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

struct LineField {
    pixels: Vec<u8>,
    flow: FlowField,
    wanderers: Vec<Wanderer>,
    ticks: u32,
}

impl LineField {
    fn new() -> Self {
        Self {
            pixels: vec![0; WIDTH * HEIGHT],
            flow: FlowField::random(),
            wanderers: (0..WANDERERS).map(|_| Wanderer::spawn()).collect(),
            ticks: 0,
        }
    }

    fn advance(&mut self, ticks: u32) {
        for wanderer in &mut self.wanderers {
            for _ in 0..STEPS_PER_TICK * ticks {
                let angle = self.flow.angle_at(wanderer.x, wanderer.y);
                wanderer.x += angle.cos() * STEP_LENGTH;
                wanderer.y += angle.sin() * STEP_LENGTH;
                wanderer.steps_left -= 1;
                if wanderer.steps_left <= 0 || !wanderer.is_inside() {
                    *wanderer = Wanderer::spawn();
                    continue;
                }

                let pixel = &mut self.pixels[wanderer.y as usize * WIDTH + wanderer.x as usize];
                *pixel = pixel
                    .saturating_add(BRIGHTNESS_PER_VISIT)
                    .min(BRIGHTEST_LINE);
            }
        }
        self.ticks += ticks;
    }

    fn is_drawn(&self) -> bool {
        self.ticks >= TICKS_UNTIL_DRAWN
    }

    fn texture(&self) -> gdk::MemoryTexture {
        gdk::MemoryTexture::new(
            WIDTH as i32,
            HEIGHT as i32,
            gdk::MemoryFormat::G8,
            &glib::Bytes::from(&self.pixels),
            WIDTH,
        )
    }
}

fn animations_enabled() -> bool {
    gtk::Settings::default().is_none_or(|settings| settings.is_gtk_enable_animations())
}

pub fn line_field_backdrop(state: &AppState) -> gtk::Picture {
    let backdrop = gtk::Picture::builder()
        .can_shrink(true)
        .content_fit(gtk::ContentFit::Cover)
        .hexpand(true)
        .vexpand(true)
        .can_target(false)
        .build();

    let unfinished_field = Rc::new(RefCell::new(Some(LineField::new())));
    let drawing: Rc<RefCell<Option<glib::SourceId>>> = Rc::default();

    backdrop.connect_map(glib::clone!(
        #[strong]
        unfinished_field,
        #[strong]
        drawing,
        move |backdrop| {
            let still_drawing_in = unfinished_field.borrow().is_some();
            let already_ticking = drawing.borrow().is_some();
            if still_drawing_in && !already_ticking {
                let ticking = draw_in(backdrop, &unfinished_field, &drawing);
                drawing.replace(Some(ticking));
            }
        }
    ));

    backdrop.connect_unmap(glib::clone!(
        #[strong]
        drawing,
        move |_| {
            if let Some(source) = drawing.take() {
                source.remove();
            }
        }
    ));

    state
        .bind_property("theme", &backdrop, "visible")
        .transform_to(|_, theme: Theme| Some(theme == Theme::Monochrome))
        .sync_create()
        .build();

    backdrop
}

fn draw_in(
    backdrop: &gtk::Picture,
    unfinished_field: &Rc<RefCell<Option<LineField>>>,
    drawing: &Rc<RefCell<Option<glib::SourceId>>>,
) -> glib::SourceId {
    let animate = animations_enabled();
    let ticks_per_frame = if animate {
        1
    } else {
        TICKS_SKIPPED_WITHOUT_ANIMATIONS
    };

    glib::timeout_add_local(
        TICK_INTERVAL,
        glib::clone!(
            #[weak]
            backdrop,
            #[strong]
            unfinished_field,
            #[strong]
            drawing,
            #[upgrade_or]
            glib::ControlFlow::Break,
            move || {
                let mut field_slot = unfinished_field.borrow_mut();
                let Some(field) = field_slot.as_mut() else {
                    drawing.take();
                    return glib::ControlFlow::Break;
                };

                field.advance(ticks_per_frame);
                let drawn = field.is_drawn();
                if animate || drawn {
                    backdrop.set_paintable(Some(&field.texture()));
                }
                if !drawn {
                    return glib::ControlFlow::Continue;
                }

                field_slot.take();
                drawing.take();
                glib::ControlFlow::Break
            }
        ),
    )
}
