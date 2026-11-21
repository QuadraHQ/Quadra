use super::icon::{self, Symbol, app_icon};
use super::logs::LiveLog;
use super::preferences::{body, caption, push_button, symbol_button, title};
use super::style::{self, DIALOG_TITLE_SIZE, Emphasis, Look};
use crate::quadra::Message;
use iced::widget::text::Wrapping;
use iced::widget::{
    Column, center, column, container, mouse_area, opaque, row, space, text, text_editor,
};
use iced::{Alignment, Element, Length};

const NARROW_WIDTH: f32 = 420.0;
const WIDE_WIDTH: f32 = 600.0;
const LOGS_WIDTH: f32 = 720.0;
const LOGS_HEIGHT: f32 = 480.0;
const EDITOR_HEIGHT: f32 = 220.0;
const STATUS_ICON_SIZE: f32 = 48.0;
const QUADRA_VERSION: &str = env!("CARGO_PKG_VERSION");
const WEBSITE: &str = "https://quadrahq.org";
const DISCORD: &str = "https://discord.gg/W8jXfngc8z";
const LICENSE: &str = "https://www.gnu.org/licenses/gpl-3.0.html";

#[derive(Debug, Clone)]
pub struct Response {
    pub label: &'static str,
    pub emphasis: Emphasis,
    pub then: Option<Message>,
}

impl Response {
    pub fn close(label: &'static str) -> Self {
        Self {
            label,
            emphasis: Emphasis::Normal,
            then: None,
        }
    }

    pub fn suggested(label: &'static str, then: Message) -> Self {
        Self {
            label,
            emphasis: Emphasis::Suggested,
            then: Some(then),
        }
    }

    pub fn destructive(label: &'static str, then: Message) -> Self {
        Self {
            label,
            emphasis: Emphasis::Destructive,
            then: Some(then),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Alert {
    pub heading: String,
    pub body: String,
    pub code: Option<String>,
    pub responses: Vec<Response>,
}

impl Alert {
    pub fn new(heading: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            heading: heading.into(),
            body: body.into(),
            code: None,
            responses: Vec::new(),
        }
    }

    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    pub fn with_response(mut self, response: Response) -> Self {
        self.responses.push(response);
        self
    }
}

pub enum Dialog {
    Alert(Alert),
    Logs,
    About,
    ImportFlags(text_editor::Content),
    ExportFlags(text_editor::Content),
}

impl Dialog {
    pub fn responses(&self) -> &[Response] {
        match self {
            Self::Alert(alert) => &alert.responses,
            Self::Logs | Self::About | Self::ImportFlags(_) | Self::ExportFlags(_) => &[],
        }
    }
}

pub fn view<'a>(dialog: &'a Dialog, live_log: &'a LiveLog, look: Look) -> Element<'a, Message> {
    let (content, width) = match dialog {
        Dialog::Alert(alert) => (alert_view(alert, look), alert_width(alert)),
        Dialog::Logs => (logs_view(live_log, look), LOGS_WIDTH),
        Dialog::About => (about_view(look), NARROW_WIDTH),
        Dialog::ImportFlags(content) => (import_view(content, look), WIDE_WIDTH),
        Dialog::ExportFlags(content) => (export_view(content, look), WIDE_WIDTH),
    };

    let card = container(content)
        .max_width(width)
        .style(style::dialog(look));
    let backdrop = mouse_area(center(opaque(card)).padding(24).style(style::shade(look)))
        .on_press(Message::DismissDialog);
    opaque(backdrop)
}

fn alert_width(alert: &Alert) -> f32 {
    if alert.code.is_some() {
        WIDE_WIDTH
    } else {
        NARROW_WIDTH
    }
}

fn alert_view<'a>(alert: &'a Alert, look: Look) -> Element<'a, Message> {
    let mut content = Column::new()
        .spacing(12)
        .padding([24, 24])
        .align_x(Alignment::Center)
        .push(title(look, &alert.heading, DIALOG_TITLE_SIZE).center())
        .push(body(look, &alert.body).center().style(style::dimmed(look)));
    if let Some(code) = &alert.code {
        content = content.push(code_block(code, look));
    }

    let responses = alert
        .responses
        .iter()
        .enumerate()
        .map(|(position, response)| {
            push_button(look, response.label, response.emphasis)
                .on_press(Message::Respond(position))
                .width(Length::Fill)
                .into()
        });
    let responses = row(responses).spacing(8).padding([0, 24]);

    column![content, responses, space().height(20)].into()
}

pub fn code_block<'a>(code: &'a str, look: Look) -> Element<'a, Message> {
    container(
        text(code)
            .font(look.mono)
            .size(13)
            .wrapping(Wrapping::Glyph),
    )
    .padding(12)
    .width(Length::Fill)
    .style(style::code_block(look))
    .into()
}

fn header<'a>(heading: &'a str, look: Look) -> Element<'a, Message> {
    row![
        title(look, heading, DIALOG_TITLE_SIZE).width(Length::Fill),
        push_button(look, "Close", Emphasis::Flat).on_press(Message::DismissDialog)
    ]
    .align_y(Alignment::Center)
    .padding([12, 16])
    .into()
}

fn logs_view<'a>(live_log: &'a LiveLog, look: Look) -> Element<'a, Message> {
    let content: Element<'a, Message> = if live_log.is_empty() {
        center(
            column![
                icon::sized(Symbol::Terminal, look.dimmed, STATUS_ICON_SIZE),
                space().height(6),
                title(look, "No Logs Yet", 20.0),
                caption(look, "Roblox's output shows up here while it runs."),
            ]
            .spacing(10)
            .align_x(Alignment::Center),
        )
        .into()
    } else {
        live_log.view(look)
    };

    column![
        header("Logs", look),
        container(content)
            .padding([0, 12])
            .height(Length::Fill)
            .max_height(LOGS_HEIGHT)
    ]
    .padding(iced::padding::bottom(12))
    .into()
}

fn about_view<'a>(look: Look) -> Element<'a, Message> {
    let link = |label: &'static str, uri: &'static str| {
        push_button(look, label, Emphasis::Normal)
            .on_press(Message::Open(uri.to_owned()))
            .width(Length::Fill)
    };

    column![
        app_icon(96.0),
        title(look, "Quadra", 22.0),
        caption(look, "QuadraHQ"),
        container(caption(look, QUADRA_VERSION).font(look.mono).style(style::colored(look.accent_text)))
            .padding([2, 10]),
        body(
            look,
            "A bootstrapper for the macOS Roblox Player and Studio, running on Linux through Rosemary."
        )
        .center(),
        column![
            link("Website", WEBSITE),
            link("Discord", DISCORD),
            link("License: GPL-3.0-only", LICENSE),
        ]
        .spacing(6),
        push_button(look, "Close", Emphasis::Flat).on_press(Message::DismissDialog),
    ]
    .spacing(10)
    .padding(24)
    .align_x(Alignment::Center)
    .into()
}

fn flags_editor<'a>(
    content: &'a text_editor::Content,
    look: Look,
    editable: bool,
) -> Element<'a, Message> {
    text_editor(content)
        .on_action(move |action| Message::DialogEditor(action, editable))
        .font(look.mono)
        .size(13)
        .height(Length::Fixed(EDITOR_HEIGHT))
        .wrapping(Wrapping::Glyph)
        .style(style::editor(look))
        .into()
}

fn import_view<'a>(content: &'a text_editor::Content, look: Look) -> Element<'a, Message> {
    column![
        title(look, "Import FastFlags", DIALOG_TITLE_SIZE),
        body(
            look,
            "Paste JSON like {\"FlagName\": value}. The flags are added to your current ones."
        )
        .style(style::dimmed(look)),
        flags_editor(content, look, true),
        row![
            push_button(look, "Cancel", Emphasis::Normal)
                .on_press(Message::DismissDialog)
                .width(Length::Fill),
            push_button(look, "Import", Emphasis::Suggested)
                .on_press(Message::ImportFlags)
                .width(Length::Fill),
        ]
        .spacing(8),
    ]
    .spacing(12)
    .padding(24)
    .into()
}

fn export_view<'a>(content: &'a text_editor::Content, look: Look) -> Element<'a, Message> {
    column![
        title(look, "Export FastFlags", DIALOG_TITLE_SIZE),
        body(look, "Copy your flags or save them to a file.").style(style::dimmed(look)),
        flags_editor(content, look, false),
        row![
            push_button(look, "Close", Emphasis::Normal)
                .on_press(Message::DismissDialog)
                .width(Length::Fill),
            symbol_button(look, Symbol::File, "Save to file…", Emphasis::Normal)
                .on_press(Message::SaveExportedFlags)
                .width(Length::Fill),
            push_button(look, "Copy", Emphasis::Suggested)
                .on_press(Message::CopyExportedFlags)
                .width(Length::Fill),
        ]
        .spacing(8),
    ]
    .spacing(12)
    .padding(24)
    .into()
}
