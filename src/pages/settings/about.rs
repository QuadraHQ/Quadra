use adw::prelude::*;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const WEBSITE: &str = "https://quadrahq.org";
const DISCORD: &str = "https://discord.gg/W8jXfngc8z";

pub fn show_about(parent: &impl IsA<gtk::Widget>) {
    let dialog = adw::AboutDialog::builder()
        .application_name("Quadra")
        .application_icon(crate::APP_ID)
        .developer_name("QuadraHQ")
        .version(VERSION)
        .website(WEBSITE)
        .license_type(gtk::License::Gpl30Only)
        .comments("A bootstrapper for the macOS Roblox Player and Studio, running on Linux through Rosemary.")
        .build();
    dialog.add_link("Discord", DISCORD);
    dialog.present(Some(parent));
}
