use super::theme_backdrop::PatternColors;
use crate::quadra::Message;
use iced::mouse::Cursor;
use iced::widget::canvas::{self, Cache, Frame, Geometry};
use iced::{Element, Length, Point, Rectangle, Renderer, Size, Theme};

const LINE_SPACING: usize = 3;
const LINE_HEIGHT: f32 = 1.0;

pub struct ScanlineBackdrop {
    cache: Cache,
    colors: PatternColors,
}

impl ScanlineBackdrop {
    pub fn new(colors: PatternColors) -> Self {
        Self {
            cache: Cache::new(),
            colors,
        }
    }

    pub fn paint_with(&mut self, colors: PatternColors) {
        if self.colors != colors {
            self.colors = colors;
            self.cache.clear();
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        canvas::Canvas::new(self)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

impl canvas::Program<Message> for ScanlineBackdrop {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
        let scanlines = self
            .cache
            .draw(renderer, bounds.size(), |frame: &mut Frame| {
                let Size { width, height } = frame.size();
                frame.fill_rectangle(Point::ORIGIN, frame.size(), self.colors.background);
                for y in (0..height as usize).step_by(LINE_SPACING) {
                    frame.fill_rectangle(
                        Point::new(0.0, y as f32),
                        Size::new(width, LINE_HEIGHT),
                        self.colors.line,
                    );
                }
            });
        vec![scanlines]
    }
}
