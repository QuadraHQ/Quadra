use super::style::{self, Look};
use super::wheel_passthrough::wheel_passthrough;
use crate::quadra::Message;
use crate::state::LogChange;
use iced::Task;
use iced::widget::operation;
use iced::widget::scrollable;
use iced::widget::scrollable::Viewport;
use iced::widget::text::Wrapping;
use iced::widget::text_editor;
use iced::widget::text_editor::{Action, Content, Edit, Motion};
use iced::{Element, Length};
use std::sync::Arc;

pub struct LiveLog {
    content: Content,
    is_following: bool,
}

impl Default for LiveLog {
    fn default() -> Self {
        Self {
            content: Content::new(),
            is_following: true,
        }
    }
}

const FOLLOWING_FROM: f32 = 0.999;
const SCROLLER: &str = "live-log";
const SCROLLBAR_GAP: f32 = 4.0;

impl LiveLog {
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    pub fn follow(&mut self, change: LogChange<'_>) -> Task<Message> {
        match change {
            LogChange::Cleared => {
                self.content = Content::new();
                self.is_following = true;
            }
            LogChange::Appended(text) => self.append(text),
        }
        if self.is_following {
            operation::snap_to_end(SCROLLER)
        } else {
            Task::none()
        }
    }

    fn append(&mut self, text: &str) {
        let reading_position = self.content.cursor();
        self.content.perform(Action::Move(Motion::DocumentEnd));
        self.content
            .perform(Action::Edit(Edit::Paste(Arc::new(text.to_owned()))));
        self.content.move_to(reading_position);
    }

    pub fn opened(&mut self) -> Task<Message> {
        self.is_following = true;
        operation::snap_to_end(SCROLLER)
    }

    pub fn scrolled(&mut self, viewport: Viewport) {
        self.is_following = viewport.relative_offset().y >= FOLLOWING_FROM
            || viewport.content_bounds().height <= viewport.bounds().height;
    }

    pub fn perform(&mut self, action: Action) {
        if !action.is_edit() {
            self.content.perform(action);
        }
    }

    pub fn view(&self, look: Look) -> Element<'_, Message> {
        let log = text_editor(&self.content)
            .on_action(Message::LogAction)
            .font(look.mono)
            .size(look.sized(12.5))
            .height(Length::Shrink)
            .wrapping(Wrapping::WordOrGlyph)
            .padding(12)
            .style(style::log(look));
        scrollable(wheel_passthrough(log))
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new().spacing(SCROLLBAR_GAP),
            ))
            .id(SCROLLER)
            .on_scroll(Message::LogScrolled)
            .height(Length::Fill)
            .style(style::scrollbar(look))
            .into()
    }
}
