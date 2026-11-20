use crate::quadra::Message;
use iced::gradient::Linear;
use iced::widget::{container, space};
use iced::{Background, Color, Degrees, Element, Gradient, Length, color};

pub struct Glow {
    angle: Degrees,
    stops: &'static [(f32, Color)],
}

pub const AURORA: Glow = Glow {
    angle: Degrees(135.0),
    stops: &[
        (0.0, color!(0x2b1552)),
        (0.5, color!(0x140d24)),
        (1.0, color!(0x3b1036)),
    ],
};

pub const EMBER: Glow = Glow {
    angle: Degrees(180.0),
    stops: &[(0.0, color!(0x2a1a10)), (0.6, color!(0x16110e))],
};

pub fn gradient_backdrop<'a>(glow: &'static Glow) -> Element<'a, Message> {
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
