mod bound_rows;
mod environment;
mod environment_variables;
mod fast_flag_allowlist;
mod fast_flags;
mod integration;
mod mangohud;
mod rosemary_options;
mod rosemary_runtime;
mod vr;

use crate::state::AppState;
use crate::utils::{scrollable, show_only_while_roblox_is_selected, unfocus_on_outside_click};
use adw::prelude::*;
use environment::env_section;
use fast_flags::fast_flags_section;
use integration::integration_section;
use vr::vr_section;

pub fn settings_page(state: &AppState) -> gtk::Widget {
    let stack = adw::ViewStack::new();
    stack.set_vexpand(true);

    stack.add_titled_with_icon(
        &scrollable(&env_section(state)),
        Some("environment"),
        "Environment",
        "emblem-system-symbolic",
    );
    let fast_flags = stack.add_titled_with_icon(
        &scrollable(&fast_flags_section(state)),
        Some("fast-flags"),
        "FastFlags",
        "emoji-flags-symbolic",
    );
    let vr = stack.add_titled_with_icon(
        &scrollable(&vr_section(state)),
        Some("vr"),
        "VR",
        "input-gaming-symbolic",
    );
    stack.add_titled_with_icon(
        &scrollable(&integration_section(state)),
        Some("integration"),
        "Integration",
        "view-app-grid-symbolic",
    );
    show_only_while_roblox_is_selected(&fast_flags, state);
    show_only_while_roblox_is_selected(&vr, state);

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
