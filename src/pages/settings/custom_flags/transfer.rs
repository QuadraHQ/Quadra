use crate::state::AppState;
use adw::prelude::*;
use gtk::{gio, glib};

const CLOSE_RESPONSE: &str = "close";
const COPY_RESPONSE: &str = "copy";
const SAVE_RESPONSE: &str = "save";
const IMPORT_RESPONSE: &str = "import";
const EXPORT_FILE_NAME: &str = "ClientAppSettings.json";

pub fn show_import(
    parent: &impl IsA<gtk::Widget>,
    state: &AppState,
    when_imported: impl Fn(&AppState) + 'static,
) {
    let (view, scroller) = text_box(true);
    let dialog = adw::AlertDialog::builder()
        .heading("Import FastFlags")
        .body("Paste JSON like {\"FlagName\": value}. The flags are added to your current ones.")
        .extra_child(&scroller)
        .build();
    dialog.add_response(CLOSE_RESPONSE, "Cancel");
    dialog.add_response(IMPORT_RESPONSE, "Import");
    dialog.set_response_appearance(IMPORT_RESPONSE, adw::ResponseAppearance::Suggested);
    dialog.set_close_response(CLOSE_RESPONSE);

    dialog.connect_response(
        Some(IMPORT_RESPONSE),
        glib::clone!(
            #[strong]
            state,
            move |_, _| {
                let text = view.buffer().text(
                    &view.buffer().start_iter(),
                    &view.buffer().end_iter(),
                    false,
                );
                match state.fast_flags().merged_with_json(&text) {
                    Ok((flags, _)) => {
                        state.set_fast_flags(flags);
                        when_imported(&state);
                    }
                    Err(error) => {
                        state.report_error(&format!("Couldn't import the flags: {error}"))
                    }
                }
            }
        ),
    );

    dialog.present(Some(parent));
}

pub fn show_export(parent: &impl IsA<gtk::Widget>, state: &AppState) {
    let json = state.fast_flags().as_json();
    let (view, scroller) = text_box(false);
    view.buffer().set_text(&json);

    let dialog = adw::AlertDialog::builder()
        .heading("Export FastFlags")
        .body("Copy your flags or save them to a file.")
        .extra_child(&scroller)
        .build();
    dialog.add_response(CLOSE_RESPONSE, "Close");
    dialog.add_response(SAVE_RESPONSE, "Save to file…");
    dialog.add_response(COPY_RESPONSE, "Copy");
    dialog.set_response_appearance(COPY_RESPONSE, adw::ResponseAppearance::Suggested);
    dialog.set_close_response(CLOSE_RESPONSE);

    dialog.connect_response(
        Some(COPY_RESPONSE),
        glib::clone!(
            #[strong]
            json,
            move |dialog, _| dialog.clipboard().set_text(&json)
        ),
    );
    let window = parent.root().and_downcast::<gtk::Window>();
    dialog.connect_response(
        Some(SAVE_RESPONSE),
        glib::clone!(
            #[strong]
            state,
            move |_, _| save_to_file(&state, window.as_ref(), json.clone())
        ),
    );

    dialog.present(Some(parent));
}

fn save_to_file(state: &AppState, window: Option<&gtk::Window>, json: String) {
    let dialog = gtk::FileDialog::builder()
        .title("Save FastFlags")
        .initial_name(EXPORT_FILE_NAME)
        .modal(true)
        .build();

    dialog.save(
        window,
        gio::Cancellable::NONE,
        glib::clone!(
            #[strong]
            state,
            move |chosen| {
                let Some(path) = chosen.ok().and_then(|file| file.path()) else {
                    return;
                };
                if let Err(error) = std::fs::write(&path, json) {
                    state.report_error(&format!("Couldn't save to {}: {error}", path.display()));
                }
            }
        ),
    );
}

fn text_box(editable: bool) -> (gtk::TextView, gtk::ScrolledWindow) {
    let view = gtk::TextView::builder()
        .monospace(true)
        .editable(editable)
        .wrap_mode(gtk::WrapMode::Char)
        .build();
    let scroller = gtk::ScrolledWindow::builder()
        .child(&view)
        .min_content_width(420)
        .min_content_height(220)
        .css_classes(["card"])
        .build();
    (view, scroller)
}
