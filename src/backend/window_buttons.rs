use super::background::run_quietly;
use crate::state::AppState;
use anyhow::{Context, bail};
use zbus::zvariant::{OwnedValue, Value};

const PORTAL: &str = "org.freedesktop.portal.Desktop";
const PORTAL_PATH: &str = "/org/freedesktop/portal/desktop";
const SETTINGS_INTERFACE: &str = "org.freedesktop.portal.Settings";
const WINDOW_PREFERENCES: &str = "org.gnome.desktop.wm.preferences";
const BUTTON_LAYOUT: &str = "button-layout";

pub fn load_layout(state: &AppState) {
    let state = state.clone();
    run_quietly(read_layout, move |layout| match layout {
        Ok(layout) => state.set_window_button_layout(layout),
        Err(error) => eprintln!("Couldn't read the window button layout: {error:#}"),
    });
}

fn read_layout() -> anyhow::Result<String> {
    async_io::block_on(async {
        let session = zbus::Connection::session()
            .await
            .context("Couldn't reach the session bus")?;
        let setting = (WINDOW_PREFERENCES, BUTTON_LAYOUT);
        let reply = match session
            .call_method(
                Some(PORTAL),
                PORTAL_PATH,
                Some(SETTINGS_INTERFACE),
                "ReadOne",
                &setting,
            )
            .await
        {
            Ok(reply) => reply,
            Err(_) => session
                .call_method(
                    Some(PORTAL),
                    PORTAL_PATH,
                    Some(SETTINGS_INTERFACE),
                    "Read",
                    &setting,
                )
                .await
                .context("The settings portal didn't answer")?,
        };
        let layout: OwnedValue = reply.body().deserialize()?;
        text_in(&layout)
    })
}

fn text_in(value: &Value<'_>) -> anyhow::Result<String> {
    match value {
        Value::Str(text) => Ok(text.as_str().to_owned()),
        Value::Value(wrapped) => text_in(wrapped),
        _ => bail!("The button layout isn't text"),
    }
}
