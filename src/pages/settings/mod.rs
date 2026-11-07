mod bound_rows;
mod environment;
mod environment_variables;
mod fast_flag_allowlist;
mod fast_flags;
mod mangohud;
mod rosemary_options;
mod rosemary_runtime;

use crate::state::AppState;
use crate::utils::{scrollable, unfocus_on_outside_click};
use adw::prelude::*;
use environment::env_section;
use fast_flags::fast_flags_section;

pub fn settings_page(state: &AppState) -> gtk::Widget {
    let stack = adw::ViewStack::new();
    stack.set_vexpand(true);

    stack.add_titled_with_icon(
        &scrollable(&env_section(state)),
        Some("environment"),
        "Environment",
        "emblem-system-symbolic",
    );
    stack.add_titled_with_icon(
        &scrollable(&fast_flags_section(state)),
        Some("fast-flags"),
        "FastFlags",
        "emoji-flags-symbolic",
    );

    let switcher = adw::ViewSwitcher::builder()
        .stack(&stack)
        .policy(adw::ViewSwitcherPolicy::Wide)
        .halign(gtk::Align::Center)
        .margin_top(12)
        .margin_bottom(12)
        .build();

    let page = gtk::Box::new(gtk::Orientation::Vertical, 0);
    page.append(&switcher);
    page.append(&stack);
    unfocus_on_outside_click(&page);

    page.upcast()
}
