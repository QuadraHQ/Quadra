use super::icon::{self, Symbol};
use super::style::{self, Look};
use crate::quadra::{Message, Page};
use iced::widget::{Row, button, container, row, text};
use iced::{Alignment, Element, Length};

pub fn bar<'a>(look: Look, shown: Page, pages: &[Page]) -> Element<'a, Message> {
    container(tabs(look, shown, pages, false))
        .padding(6)
        .center_x(Length::Fill)
        .style(style::bar(look))
        .into()
}

pub fn tabs<'a>(look: Look, shown: Page, pages: &[Page], compact: bool) -> Row<'a, Message> {
    row(pages.iter().map(|&page| {
        let is_shown = page == shown;
        let color = if is_shown {
            look.selected_text()
        } else {
            look.text
        };
        let label = (!compact).then(|| text(page.title()).font(look.font).size(14));
        button(
            row![icon::tinted(symbol_of(page), color)]
                .push(label)
                .spacing(8)
                .align_y(Alignment::Center),
        )
        .padding(if compact { [6, 10] } else { [6, 16] })
        .style(style::tab(look, is_shown))
        .on_press(Message::ShowPage(page))
        .into()
    }))
    .spacing(4)
}

fn symbol_of(page: Page) -> Symbol {
    match page {
        Page::Home => Symbol::Home,
        Page::Servers => Symbol::Server,
        Page::Settings => Symbol::Settings,
    }
}
