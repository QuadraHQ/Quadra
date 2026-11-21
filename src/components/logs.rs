use super::style::{self, Look};
use crate::quadra::Message;
use crate::state::LogChange;
use iced::widget::text::Wrapping;
use iced::widget::text_editor;
use iced::widget::text_editor::{Action, Content, Edit, Motion};
use iced::{Element, Length};
use std::sync::Arc;

#[derive(Default)]
pub struct LiveLog {
    content: Content,
}

impl LiveLog {
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    pub fn follow(&mut self, change: LogChange<'_>) {
        match change {
            LogChange::Cleared => self.content = Content::new(),
            LogChange::Appended(text) => self.append(text),
        }
    }

    fn append(&mut self, text: &str) {
        let was_following_output = self.is_at_end();
        let reading_position = self.content.cursor();

        self.content.perform(Action::Move(Motion::DocumentEnd));
        self.content
            .perform(Action::Edit(Edit::Paste(Arc::new(text.to_owned()))));

        if !was_following_output {
            self.content.move_to(reading_position);
        }
    }

    fn is_at_end(&self) -> bool {
        let cursor = self.content.cursor();
        let last_line = self.content.line_count().saturating_sub(1);
        let last_line_length = self
            .content
            .line(last_line)
            .map_or(0, |line| line.text.len());
        cursor.selection.is_none()
            && cursor.position.line == last_line
            && cursor.position.column == last_line_length
    }

    pub fn perform(&mut self, action: Action) {
        if !action.is_edit() {
            self.content.perform(action);
        }
    }

    pub fn view(&self, look: Look) -> Element<'_, Message> {
        text_editor(&self.content)
            .on_action(Message::LogAction)
            .font(look.mono)
            .size(look.sized(12.5))
            .height(Length::Fill)
            .wrapping(Wrapping::WordOrGlyph)
            .padding(12)
            .style(style::log(look))
            .into()
    }
}
