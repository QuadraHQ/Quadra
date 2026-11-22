use super::style::{self, Look};
use crate::quadra::Message;
use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::Widget;
use iced::advanced::widget::tree::{self, Tree};
use iced::advanced::{Clipboard, Shell, mouse, renderer};
use iced::time::{Duration, Instant};
use iced::widget::toggler;
use iced::{
    Background, Border, Color, Element, Event, Length, Rectangle, Renderer, Size, Theme, window,
};

const HEIGHT: f32 = 22.0;
const KNOB_INSET: f32 = 2.0;
const OUTLINED_SIZE: Size = Size::new(32.0, 15.0);
const OUTLINED_KNOB: Size = Size::new(13.0, 9.0);
const OUTLINED_INSET: f32 = 3.0;
const SLIDING_TIME: f32 = 0.15;
const DISABLED_OPACITY: f32 = 0.45;
const LONGEST_STEP: f32 = 0.05;
const SLIDING_FRAME: Duration = Duration::from_millis(8);

pub fn switch<'a>(
    look: Look,
    is_on: bool,
    on_toggle: Option<Box<dyn Fn(bool) -> Message + 'a>>,
) -> Element<'a, Message> {
    Element::new(Switch {
        look,
        is_on,
        on_toggle,
    })
}

struct Switch<'a> {
    look: Look,
    is_on: bool,
    on_toggle: Option<Box<dyn Fn(bool) -> Message + 'a>>,
}

struct Slide {
    position: f32,
    last_frame: Option<Instant>,
}

impl Slide {
    fn wanted(is_on: bool) -> f32 {
        if is_on { 1.0 } else { 0.0 }
    }

    fn advance(&mut self, now: Instant, is_on: bool, animates: bool) -> bool {
        let wanted = Self::wanted(is_on);
        if !animates {
            self.position = wanted;
            self.last_frame = None;
            return false;
        }
        let step = self.last_frame.map_or(0.0, |last_frame| {
            now.saturating_duration_since(last_frame)
                .as_secs_f32()
                .min(LONGEST_STEP)
        });
        let change = step / SLIDING_TIME;
        self.position = if self.position < wanted {
            (self.position + change).min(wanted)
        } else {
            (self.position - change).max(wanted)
        };
        if self.position == wanted {
            self.last_frame = None;
            return false;
        }
        self.last_frame = Some(now);
        true
    }
}

fn eased(position: f32) -> f32 {
    position * position * (3.0 - 2.0 * position)
}

fn mixed(from: Color, to: Color, amount: f32) -> Color {
    Color::from_rgba(
        from.r + (to.r - from.r) * amount,
        from.g + (to.g - from.g) * amount,
        from.b + (to.b - from.b) * amount,
        from.a + (to.a - from.a) * amount,
    )
}

fn colour_of(background: Background) -> Color {
    match background {
        Background::Color(color) => color,
        Background::Gradient(_) => Color::TRANSPARENT,
    }
}

impl Switch<'_> {
    fn is_outlined(&self) -> bool {
        self.look.radius == 0.0
    }

    fn drawn_size(&self) -> Size {
        if self.is_outlined() {
            OUTLINED_SIZE
        } else {
            Size::new(HEIGHT * 2.0, HEIGHT)
        }
    }

    fn draw_outlined(
        &self,
        renderer: &mut Renderer,
        bounds: Rectangle,
        position: f32,
        is_hovered: bool,
    ) {
        use iced::advanced::Renderer as _;
        let look = self.look;
        let resting = if is_hovered { look.text } else { look.dimmed };
        let edge = mixed(resting, look.accent_text, position);
        let fill = Color {
            a: look.pressed.a * position,
            ..look.pressed
        };
        let fade = |color: Color| {
            if self.on_toggle.is_some() {
                color
            } else {
                Color {
                    a: color.a * DISABLED_OPACITY,
                    ..color
                }
            }
        };
        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border: Border {
                    color: fade(edge),
                    width: 1.0,
                    radius: 0.0.into(),
                },
                ..renderer::Quad::default()
            },
            fade(fill),
        );
        let travel = bounds.width - OUTLINED_KNOB.width - OUTLINED_INSET * 2.0;
        renderer.fill_quad(
            renderer::Quad {
                bounds: Rectangle {
                    x: bounds.x + OUTLINED_INSET + travel * position,
                    y: bounds.y + (bounds.height - OUTLINED_KNOB.height) / 2.0,
                    width: OUTLINED_KNOB.width,
                    height: OUTLINED_KNOB.height,
                },
                ..renderer::Quad::default()
            },
            fade(edge),
        );
    }
}

impl Widget<Message, Theme, Renderer> for Switch<'_> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Slide>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Slide {
            position: Slide::wanted(self.is_on),
            last_frame: None,
        })
    }

    fn size(&self) -> Size<Length> {
        let size = self.drawn_size();
        Size::new(Length::Fixed(size.width), Length::Fixed(size.height))
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let size = self.drawn_size();
        layout::Node::new(limits.resolve(
            Length::Fixed(size.width),
            Length::Fixed(size.height),
            size,
        ))
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let slide = tree.state.downcast_mut::<Slide>();
        match event {
            Event::Window(window::Event::RedrawRequested(now)) => {
                if slide.position != Slide::wanted(self.is_on)
                    && slide.advance(*now, self.is_on, self.look.animates)
                {
                    shell.request_redraw_at(*now + SLIDING_FRAME);
                }
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                if cursor.is_over(layout.bounds()) =>
            {
                if let Some(on_toggle) = &self.on_toggle {
                    shell.publish(on_toggle(!self.is_on));
                    shell.capture_event();
                }
            }
            _ => {}
        }
        if slide.position != Slide::wanted(self.is_on) && slide.last_frame.is_none() {
            shell.request_redraw();
        }
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if self.on_toggle.is_some() && cursor.is_over(layout.bounds()) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        use iced::advanced::Renderer as _;
        let position = eased(tree.state.downcast_ref::<Slide>().position);
        let bounds = layout.bounds();
        let is_hovered = cursor.is_over(bounds);
        if self.is_outlined() {
            self.draw_outlined(renderer, bounds, position, is_hovered);
            return;
        }
        let painted = style::switch(self.look);
        let status = |is_toggled| match (self.on_toggle.is_some(), is_hovered) {
            (false, _) => toggler::Status::Disabled { is_toggled },
            (true, true) => toggler::Status::Hovered { is_toggled },
            (true, false) => toggler::Status::Active { is_toggled },
        };
        let off = painted(theme, status(false));
        let on = painted(theme, status(true));
        let track = mixed(
            colour_of(off.background),
            colour_of(on.background),
            position,
        );
        let knob = mixed(
            colour_of(off.foreground),
            colour_of(on.foreground),
            position,
        );
        let rounding = if self.look.radius > 0.0 {
            bounds.height / 2.0
        } else {
            0.0
        };
        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border: Border {
                    radius: rounding.into(),
                    ..Border::default()
                },
                ..renderer::Quad::default()
            },
            track,
        );
        let knob_size = bounds.height - KNOB_INSET * 2.0;
        let travel = bounds.width - knob_size - KNOB_INSET * 2.0;
        renderer.fill_quad(
            renderer::Quad {
                bounds: Rectangle {
                    x: bounds.x + KNOB_INSET + travel * position,
                    y: bounds.y + KNOB_INSET,
                    width: knob_size,
                    height: knob_size,
                },
                border: Border {
                    radius: (rounding - KNOB_INSET).max(0.0).into(),
                    ..Border::default()
                },
                ..renderer::Quad::default()
            },
            knob,
        );
    }
}
