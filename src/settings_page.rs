use crate::utils::{scrollable, unfocus_on_outside_click};
use adw::prelude::*;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn settings_page() -> gtk::Widget {
    let stack = adw::ViewStack::new();
    stack.set_vexpand(true);

    stack.add_titled_with_icon(
        &scrollable(&env_section()),
        Some("environment"),
        "Environment",
        "emblem-system-symbolic",
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

fn env_section() -> gtk::Box {
    // Quadra group
    let check_update_btn = gtk::Button::builder()
        .label("Check for updates")
        .valign(gtk::Align::Center)
        .build();
    check_update_btn.connect_clicked(|_| println!("install clicked"));

    let launcher_info = adw::ActionRow::builder()
        .title("Launcher version")
        .subtitle(format!("v{}", VERSION))
        .build();
    launcher_info.add_suffix(&check_update_btn);

    let force_update_btn = adw::ButtonRow::builder().title("Force update").build();

    let quadra_group = adw::PreferencesGroup::builder().title("Quadra").build();
    quadra_group.add(&launcher_info);
    quadra_group.add(&force_update_btn);

    // Runtime selection
    let runtime_model = gtk::StringList::new(&["Rosemary", "Darling"]);

    let runtime_row = adw::ComboRow::builder()
        .title("Run Roblox with")
        .model(&runtime_model)
        .build();

    let runtime_group = adw::PreferencesGroup::builder()
        .title("Runtime")
        .description("Which compatibility layer the macOS Roblox client runs on.")
        .build();
    runtime_group.add(&runtime_row);

    // Runtime settings
    let opengl_row = adw::SwitchRow::builder()
        .title("Force OpenGL")
        .subtitle("For machines without a GPU that supports Vulkan")
        .build();

    let refresh_rate_input = gtk::Entry::builder()
        .placeholder_text("The monitor's")
        .valign(gtk::Align::Center)
        .build();
    let refresh_rate_row = adw::ActionRow::builder()
        .title("Refresh rate (Hz)")
        .subtitle("The refresh rate Roblox is told the display has. Default's to the monitor's refresh rate.")
        .build();
    refresh_rate_row.add_suffix(&refresh_rate_input);

    let options_group = adw::PreferencesGroup::builder().title("Options").build();
    options_group.add(&opengl_row);
    options_group.add(&refresh_rate_row);

    let section = gtk::Box::new(gtk::Orientation::Vertical, 12);
    section.append(&quadra_group);
    section.append(&runtime_group);
    section.append(&options_group);

    section
}
