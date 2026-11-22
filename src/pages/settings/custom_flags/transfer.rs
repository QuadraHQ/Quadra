use crate::state::AppState;
use iced::widget::text_editor;

pub const EXPORT_FILE_NAME: &str = "ClientAppSettings.json";

pub fn import_flags(state: &AppState, json: &str) -> bool {
    match state.fast_flags().merged_with_json(json) {
        Ok((flags, _)) => {
            state.set_fast_flags(flags);
            true
        }
        Err(error) => {
            state.report_error(&format!("Couldn't import the flags: {error}"));
            false
        }
    }
}

pub fn export_dialog(state: &AppState) -> text_editor::Content {
    text_editor::Content::with_text(&state.fast_flags().as_json())
}

pub fn save_exported(state: &AppState, path: &std::path::Path, json: &str) {
    if let Err(error) = std::fs::write(path, json) {
        state.report_error(&format!("Couldn't save to {}: {error}", path.display()));
    }
}
