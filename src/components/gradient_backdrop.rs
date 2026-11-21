use crate::quadra::Message;
use iced::gradient::Linear;
use iced::widget::{container, space};
use iced::{Background, Color, Degrees, Element, Gradient, Length, color};
use std::borrow::Cow;

#[derive(Debug, Clone, PartialEq)]
pub struct Glow {
    angle: Degrees,
    stops: Cow<'static, [(f32, Color)]>,
}

impl Glow {
    pub fn new(angle: Degrees, stops: Vec<(f32, Color)>) -> Self {
        Self {
            angle,
            stops: Cow::Owned(stops),
        }
    }
}

pub const AURORA: Glow = Glow {
    angle: Degrees(135.0),
    stops: Cow::Borrowed(&[
        (0.0, color!(0x2b1552)),
        (0.5, color!(0x140d24)),
        (1.0, color!(0x3b1036)),
    ]),
};

pub const EMBER: Glow = Glow {
    angle: Degrees(180.0),
    stops: Cow::Borrowed(&[(0.0, color!(0x2a1a10)), (0.6, color!(0x16110e))]),
};

pub const PRISM: Glow = Glow {
    angle: Degrees(150.0),
    stops: Cow::Borrowed(&[
        (0.0, color!(0x1a1c44)),
        (0.55, color!(0x0d0e22)),
        (1.0, color!(0x14102e)),
    ]),
};

pub fn gradient_backdrop<'a>(glow: &Glow) -> Element<'a, Message> {
    let gradient = glow
        .stops
        .iter()
        .fold(Linear::new(glow.angle), |gradient, &(offset, color)| {
            gradient.add_stop(offset, color)
        });
    container(space().width(Length::Fill).height(Length::Fill))
        .style(move |_| container::Style {
            background: Some(Background::Gradient(Gradient::Linear(gradient))),
            ..container::Style::default()
        })
        .into()
}
