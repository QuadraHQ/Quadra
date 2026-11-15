mod about;
mod account;
mod advanced;
mod bound_rows;
mod butter_access;
mod custom_flags;
mod discord_access;
mod environment_variables;
mod fast_flag_allowlist;
mod fast_flags;
mod general;
mod integration;
mod launch_wrapper;
mod mangohud;
mod mods;
mod network;
mod performance;
mod privacy;
mod rosemary_options;
mod rosemary_runtime;
mod runtime;
mod session;
mod support;
mod updates;
mod vr;

use crate::state::AppState;
use crate::utils::{
    is_running_in_flatpak, scrollable, show_only_while_roblox_is_selected, unfocus_on_outside_click,
};
use adw::prelude::*;
use advanced::advanced_section;
use general::general_section;
use integration::integration_section;
use mods::mods_section;
use network::network_section;
use performance::performance_section;
use runtime::runtime_section;
use vr::vr_section;

pub fn settings_page(state: &AppState) -> gtk::Widget {
    let stack = adw::ViewStack::new();
    stack.set_vexpand(true);

    stack.add_titled_with_icon(
        &scrollable(&general_section(state)),
        Some("general"),
        "General",
        "preferences-system-symbolic",
    );
    stack.add_titled_with_icon(
        &scrollable(&runtime_section(state)),
        Some("runtime"),
        "Runtime",
        "computer-symbolic",
    );
    let network = stack.add_titled_with_icon(
        &scrollable(&network_section(state)),
        Some("network"),
        "Network",
        "network-wired-symbolic",
    );
    let performance = stack.add_titled_with_icon(
        &scrollable(&performance_section(state)),
        Some("performance"),
        "Performance",
        "speedometer-symbolic",
    );
    let mods = stack.add_titled_with_icon(
        &scrollable(&mods_section(state)),
        Some("mods"),
        "Mods",
        "application-x-addon-symbolic",
    );
    let vr = stack.add_titled_with_icon(
        &scrollable(&vr_section(state)),
        Some("vr"),
        "VR",
        "input-gaming-symbolic",
    );
    if !is_running_in_flatpak() {
        stack.add_titled_with_icon(
            &scrollable(&integration_section(state)),
            Some("integration"),
            "Integration",
            "view-app-grid-symbolic",
        );
    }
    let advanced_content = scrollable(&advanced_section(state));
    stack.add_titled_with_icon(
        &advanced_content,
        Some("advanced"),
        "Advanced",
        "preferences-dev-symbolic",
    );

    let advanced_page = stack.page(&advanced_content);
    state
        .bind_property("advanced-settings-visible", &advanced_page, "visible")
        .sync_create()
        .build();

    show_only_while_roblox_is_selected(&performance, state);
    show_only_while_roblox_is_selected(&mods, state);
    show_only_while_roblox_is_selected(&vr, state);
    show_only_while_roblox_is_selected(&network, state);

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
