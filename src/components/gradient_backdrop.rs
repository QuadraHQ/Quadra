use crate::quadra::Message;
use iced::gradient::Linear;
use iced::widget::{container, space};
use iced::{Background, Color, Degrees, Element, Gradient, Length};
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
