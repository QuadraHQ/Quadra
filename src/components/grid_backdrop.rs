use super::theme_backdrop::PatternColors;
use crate::quadra::Message;
use iced::mouse::Cursor;
use iced::widget::canvas::{self, Cache, Frame, Geometry};
use iced::{Element, Length, Point, Rectangle, Renderer, Size, Theme};

const CELL_SIZE: usize = 32;
const LINE_WIDTH: f32 = 1.0;

pub struct GridBackdrop {
    cache: Cache,
    colors: PatternColors,
}

impl GridBackdrop {
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

impl canvas::Program<Message> for GridBackdrop {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
        let grid = self
            .cache
            .draw(renderer, bounds.size(), |frame: &mut Frame| {
                let Size { width, height } = frame.size();
                frame.fill_rectangle(Point::ORIGIN, frame.size(), self.colors.background);
                for x in (0..width as usize).step_by(CELL_SIZE) {
                    frame.fill_rectangle(
                        Point::new(x as f32, 0.0),
                        Size::new(LINE_WIDTH, height),
                        self.colors.line,
                    );
                }
                for y in (0..height as usize).step_by(CELL_SIZE) {
                    frame.fill_rectangle(
                        Point::new(0.0, y as f32),
                        Size::new(width, LINE_WIDTH),
                        self.colors.line,
                    );
                }
            });
        vec![grid]
    }
}
