use super::background::run_quietly;
use crate::state::AppState;
use anyhow::{Context, bail};
use zbus::zvariant::{OwnedValue, Value};

const PORTAL: &str = "org.freedesktop.portal.Desktop";
const PORTAL_PATH: &str = "/org/freedesktop/portal/desktop";
const SETTINGS_INTERFACE: &str = "org.freedesktop.portal.Settings";
const BUTTON_LAYOUT: (&str, &str) = ("org.gnome.desktop.wm.preferences", "button-layout");
const REDUCED_MOTION: (&str, &str) = ("org.freedesktop.appearance", "reduced-motion");
const ENABLE_ANIMATIONS: (&str, &str) = ("org.gnome.desktop.interface", "enable-animations");

pub fn load_layout(state: &AppState) {
    let state = state.clone();
    run_quietly(read_layout, move |layout| match layout {
        Ok(layout) => state.set_window_button_layout(layout),
        Err(error) => eprintln!("Couldn't read the window button layout: {error:#}"),
    });
}

pub fn load_motion_preference(state: &AppState) {
    let state = state.clone();
    run_quietly(read_reduces_motion, move |reduces| match reduces {
        Ok(reduces) => state.set_system_reduces_motion(reduces),
        Err(error) => eprintln!("Couldn't read the desktop's animation preference: {error:#}"),
    });
}

fn read_layout() -> anyhow::Result<String> {
    async_io::block_on(async {
        let session = session().await?;
        let layout = read_setting(&session, BUTTON_LAYOUT).await?;
        match unwrapped(&layout) {
            Value::Str(text) => Ok(text.as_str().to_owned()),
            _ => bail!("The button layout isn't text"),
        }
    })
}

fn read_reduces_motion() -> anyhow::Result<bool> {
    async_io::block_on(async {
        let session = session().await?;
        if let Ok(preference) = read_setting(&session, REDUCED_MOTION).await
            && let Value::U32(reduce) = unwrapped(&preference)
        {
            return Ok(*reduce == 1);
        }
        let enabled = read_setting(&session, ENABLE_ANIMATIONS).await?;
        match unwrapped(&enabled) {
            Value::Bool(enabled) => Ok(!enabled),
            _ => bail!("The animation preference isn't a switch"),
        }
    })
}

async fn session() -> anyhow::Result<zbus::Connection> {
    zbus::Connection::session()
        .await
        .context("Couldn't reach the session bus")
}

async fn read_setting(
    session: &zbus::Connection,
    setting: (&str, &str),
) -> anyhow::Result<Value<'static>> {
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
    let value: OwnedValue = reply.body().deserialize()?;
    Ok(Value::from(value))
}

fn unwrapped<'a>(value: &'a Value<'a>) -> &'a Value<'a> {
    match value {
        Value::Value(wrapped) => unwrapped(wrapped),
        other => other,
    }
}
