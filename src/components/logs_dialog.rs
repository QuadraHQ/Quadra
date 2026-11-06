use crate::state::AppState;
use adw::prelude::*;
use gtk::glib;

const EMPTY_PAGE: &str = "empty";
const LOG_PAGE: &str = "log";
const DISTANCE_THAT_STILL_COUNTS_AS_BOTTOM: f64 = 32.0;

#[derive(Clone)]
struct LiveLog {
    pages: gtk::Stack,
    scrolled_window: gtk::ScrolledWindow,
    text_view: gtk::TextView,
    buffer: gtk::TextBuffer,
    end: gtk::TextMark,
}

impl LiveLog {
    fn new() -> Self {
        let buffer = gtk::TextBuffer::new(None);
        let end = buffer.create_mark(None, &buffer.end_iter(), false);

        let text_view = gtk::TextView::builder()
            .buffer(&buffer)
            .editable(false)
            .cursor_visible(false)
            .monospace(true)
            .wrap_mode(gtk::WrapMode::WordChar)
            .top_margin(12)
            .bottom_margin(12)
            .left_margin(12)
            .right_margin(12)
            .build();

        let scrolled_window = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .child(&text_view)
            .vexpand(true)
            .build();

        let empty_page = adw::StatusPage::builder()
            .icon_name("utilities-terminal-symbolic")
            .title("No Logs Yet")
            .description("Roblox's output shows up here while it runs.")
            .build();

        let pages = gtk::Stack::new();
        pages.add_named(&empty_page, Some(EMPTY_PAGE));
        pages.add_named(&scrolled_window, Some(LOG_PAGE));

        Self {
            pages,
            scrolled_window,
            text_view,
            buffer,
            end,
        }
    }

    fn replace(&self, text: &str) {
        self.buffer.set_text(text);
        self.show_matching_page();
        self.scroll_to_end();
    }

    fn append(&self, text: &str) {
        let was_following_output = self.is_scrolled_to_bottom();
        self.buffer.insert(&mut self.buffer.end_iter(), text);
        self.show_matching_page();
        if was_following_output {
            self.scroll_to_end();
        }
    }

    fn is_scrolled_to_bottom(&self) -> bool {
        let adjustment = self.scrolled_window.vadjustment();
        let distance_from_bottom =
            adjustment.upper() - (adjustment.value() + adjustment.page_size());
        distance_from_bottom <= DISTANCE_THAT_STILL_COUNTS_AS_BOTTOM
    }

    fn scroll_to_end(&self) {
        self.text_view
            .scroll_to_mark(&self.end, 0.0, false, 0.0, 1.0);
    }

    fn show_matching_page(&self) {
        let page = if self.buffer.char_count() == 0 {
            EMPTY_PAGE
        } else {
            LOG_PAGE
        };
        self.pages.set_visible_child_name(page);
    }
}

pub fn logs_dialog(state: &AppState) -> adw::Dialog {
    let live_log = LiveLog::new();
    live_log.replace(&state.log());

    let appended = state.connect_log_appended(glib::clone!(
        #[strong]
        live_log,
        move |_, text| live_log.append(text)
    ));
    let cleared = state.connect_log_cleared(glib::clone!(
        #[strong]
        live_log,
        move |_| live_log.replace("")
    ));

    let toolbar_view = adw::ToolbarView::new();
    toolbar_view.add_top_bar(&adw::HeaderBar::new());
    toolbar_view.set_content(Some(&live_log.pages));

    let dialog = adw::Dialog::builder()
        .title("Logs")
        .content_width(720)
        .content_height(480)
        .child(&toolbar_view)
        .build();

    let handlers = std::cell::Cell::new(Some([appended, cleared]));
    dialog.connect_closed(glib::clone!(
        #[strong]
        state,
        move |_| {
            for handler in handlers.take().into_iter().flatten() {
                state.disconnect(handler);
            }
        }
    ));

    dialog
}
