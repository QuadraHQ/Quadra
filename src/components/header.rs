use super::icon::{self, Symbol};
use super::style::{self, Look};
use crate::quadra::{Message, Page};
use iced::widget::{button, container, row, text};
use iced::{Alignment, Element, Length};

pub fn bar<'a>(look: Look, shown: Page, pages: &[Page]) -> Element<'a, Message> {
    let tabs = pages.iter().map(|&page| {
        let is_shown = page == shown;
        let color = if is_shown {
            look.accent_text
        } else {
            look.text
        };
        button(
            row![
                icon::tinted(symbol_of(page), color),
                text(page.title()).font(look.font).size(14)
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .padding([6, 16])
        .style(style::tab(look, is_shown))
        .on_press(Message::ShowPage(page))
        .into()
    });

    container(row(tabs).spacing(4))
        .padding(6)
        .center_x(Length::Fill)
        .style(style::bar(look))
        .into()
}

fn symbol_of(page: Page) -> Symbol {
    match page {
        Page::Home => Symbol::Home,
        Page::Servers => Symbol::Server,
        Page::Settings => Symbol::Settings,
    }
}
