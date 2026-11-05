use adw::prelude::*;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn settings_page() -> gtk::Widget {
    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);

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

    let delete_row = adw::ButtonRow::builder().title("Force update").build();

    let quadra_group = adw::PreferencesGroup::builder().title("Quadra").build();
    quadra_group.add(&launcher_info);
    quadra_group.add(&delete_row);

    content.append(&quadra_group);

    let clamp = adw::Clamp::builder()
        .maximum_size(600)
        .child(&content)
        .build();

    gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&clamp)
        .vexpand(true)
        .build()
        .upcast()
}
