use crate::backend;
use crate::state::AppState;
use adw::prelude::*;
use gtk::glib;

pub fn update_banner(state: &AppState) -> adw::Banner {
    let banner = adw::Banner::builder().button_label("Update").build();
    banner.connect_button_clicked(glib::clone!(
        #[strong]
        state,
        move |_| backend::install_update(&state)
    ));

    show_update(&banner, state);
    for property in ["available-update", "busy"] {
        state.connect_notify_local(
            Some(property),
            glib::clone!(
                #[weak]
                banner,
                move |state, _| show_update(&banner, state)
            ),
        );
    }

    banner
}

fn show_update(banner: &adw::Banner, state: &AppState) {
    let version = state.available_update();
    banner.set_title(&format!("Quadra {version} is available"));
    banner.set_revealed(!version.is_empty());
    banner.set_sensitive(!state.busy());
}
