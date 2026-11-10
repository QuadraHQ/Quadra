mod history;
mod notes;
mod server;
mod settings;

use crate::state::{AppState, PanelId};

pub fn content_of(id: PanelId, state: &AppState) -> gtk::Widget {
    match id {
        PanelId::Server => server::server_content(state),
        PanelId::History => history::history_content(state),
        PanelId::Notes => notes::notes_content(),
        PanelId::Settings => settings::settings_content(state),
    }
}
