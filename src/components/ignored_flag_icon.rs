use super::icon::{self, Symbol};
use super::preferences::with_tooltip;
use super::style::Look;
use crate::backend;
use crate::quadra::Message;
use iced::widget::{container, space};
use iced::{Element, Length};

const ICON_SIZE: f32 = 16.0;

pub fn ignored_flag_icon<'a>(name: &str, look: Look) -> Element<'a, Message> {
    let name = name.trim();
    let is_ignored = !name.is_empty() && backend::flag_is_ignored(name);
    let icon: Element<'a, Message> = if is_ignored {
        with_tooltip(
            look,
            icon::tinted(Symbol::Warning, look.warning),
            "Roblox ignores this FastFlag, so it has no effect",
        )
    } else {
        space().into()
    };
    container(icon)
        .width(Length::Fixed(ICON_SIZE))
        .height(Length::Fixed(ICON_SIZE))
        .into()
}
