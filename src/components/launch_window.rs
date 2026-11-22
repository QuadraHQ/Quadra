use super::icon::app_icon;
use super::preferences::{caption, title};
use super::roblox_status::status_text;
use super::style::{self, Look};
use crate::quadra::Message;
use crate::state::AppState;
use iced::widget::{column, container, progress_bar, row, space};
use iced::{Alignment, Element, Length, Size};

pub const SIZE: Size = Size::new(360.0, 280.0);
const PULSE_STEPS: u32 = 40;
const PULSE_WIDTH: u16 = 10;

pub fn launch_finished(state: &AppState) -> bool {
    let roblox_is_open_or_gone = state.roblox_graphics_started() || !state.running();
    !state.busy() && roblox_is_open_or_gone
}

pub fn has_measurable_progress(state: &AppState) -> bool {
    state.task_progress() > 0.0
}

pub fn starting_text(state: &AppState) -> String {
    format!("Starting {}", state.current_client().name())
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Pulse(u32);

impl Pulse {
    pub fn advance(&mut self) {
        self.0 = (self.0 + 1) % (PULSE_STEPS * 2);
    }

    fn offset(self) -> u16 {
        let position = if self.0 < PULSE_STEPS {
            self.0
        } else {
            PULSE_STEPS * 2 - self.0
        };
        (position * u32::from(100 - PULSE_WIDTH) / PULSE_STEPS) as u16
    }
}

pub fn view<'a>(state: &AppState, pulse: Pulse) -> Element<'a, Message> {
    let look = Look::of(state);
    let progress: Element<'a, Message> = if has_measurable_progress(state) {
        progress_bar(0.0..=1.0, state.task_progress() as f32)
            .girth(6)
            .style(style::progress(look))
            .into()
    } else {
        pulsing_bar(look, pulse)
    };

    container(
        column![
            app_icon(64.0),
            title(look, starting_text(state), 20.0),
            caption(look, status_text(state, state.current_client())).center(),
            progress,
        ]
        .spacing(12)
        .align_x(Alignment::Center),
    )
    .padding(32)
    .center(Length::Fill)
    .into()
}

fn pulsing_bar<'a>(look: Look, pulse: Pulse) -> Element<'a, Message> {
    let offset = pulse.offset();
    let block = container(space().height(6))
        .width(Length::FillPortion(PULSE_WIDTH))
        .style(move |_| container::Style {
            background: Some(look.accent.into()),
            border: iced::Border {
                radius: (look.radius * 0.5).into(),
                ..Default::default()
            },
            ..Default::default()
        });
    let trough = row![
        space().width(Length::FillPortion(offset.max(1))),
        block,
        space().width(Length::FillPortion((100 - PULSE_WIDTH - offset).max(1))),
    ];
    container(trough)
        .width(Length::Fill)
        .style(move |_| container::Style {
            background: Some(look.pressed.into()),
            border: iced::Border {
                radius: (look.radius * 0.5).into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}
