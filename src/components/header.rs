use super::icon::app_icon;
use adw::{HeaderBar, ViewStack, ViewSwitcher, ViewSwitcherPolicy, prelude::*};

pub fn bar(stack: &ViewStack) -> HeaderBar {
    let switcher = ViewSwitcher::builder()
        .stack(&stack)
        .policy(ViewSwitcherPolicy::Wide)
        .build();

    let header = HeaderBar::new();
    header.add_css_class("app-header");

    header.pack_start(&app_icon());
    header.set_title_widget(Some(&switcher));

    header
}
