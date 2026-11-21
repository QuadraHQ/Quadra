use crate::quadra::Message;
use iced::mouse::Cursor;
use iced::widget::canvas::{self, Cache, Frame, Geometry};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Size, Theme, color};

const LINE_SPACING: usize = 3;
const LINE_HEIGHT: f32 = 1.0;
const BACKGROUND: Color = color!(0x030805);
const LINE: Color = Color::from_rgba(57.0 / 255.0, 1.0, 122.0 / 255.0, 0.035);

#[derive(Default)]
pub struct ScanlineBackdrop {
    cache: Cache,
}

impl ScanlineBackdrop {
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
                frame.fill_rectangle(Point::ORIGIN, frame.size(), BACKGROUND);
                for y in (0..height as usize).step_by(LINE_SPACING) {
                    frame.fill_rectangle(
                        Point::new(0.0, y as f32),
                        Size::new(width, LINE_HEIGHT),
                        LINE,
                    );
                }
            });
        vec![scanlines]
    }
}
