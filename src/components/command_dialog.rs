use adw::prelude::*;

const COPY_RESPONSE: &str = "copy";
const CLOSE_RESPONSE: &str = "close";
const CODE_PADDING: i32 = 12;

pub fn show_command_dialog(
    parent: &impl IsA<gtk::Widget>,
    heading: &str,
    body: &str,
    command: &str,
) {
    let dialog = adw::AlertDialog::builder()
        .heading(heading)
        .body(body)
        .prefer_wide_layout(true)
        .extra_child(&code_block(command))
        .build();
    dialog.add_response(CLOSE_RESPONSE, "Close");
    dialog.add_response(COPY_RESPONSE, "Copy command");
    dialog.set_response_appearance(COPY_RESPONSE, adw::ResponseAppearance::Suggested);
    dialog.set_default_response(Some(COPY_RESPONSE));
    dialog.set_close_response(CLOSE_RESPONSE);

    let command = command.to_owned();
    dialog.connect_response(Some(COPY_RESPONSE), move |dialog, _| {
        dialog.clipboard().set_text(&command);
    });

    dialog.present(Some(parent));
}

fn code_block(code: &str) -> gtk::Box {
    let label = gtk::Label::builder()
        .label(code)
        .selectable(true)
        .wrap(true)
        .wrap_mode(gtk::pango::WrapMode::Char)
        .xalign(0.0)
        .css_classes(["monospace"])
        .margin_top(CODE_PADDING)
        .margin_bottom(CODE_PADDING)
        .margin_start(CODE_PADDING)
        .margin_end(CODE_PADDING)
        .build();
    let block = gtk::Box::builder().css_classes(["card"]).build();
    block.append(&label);
    block
}
