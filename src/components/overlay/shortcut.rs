use ashpd::WindowIdentifier;
use ashpd::desktop::CreateSessionOptions;
use ashpd::desktop::global_shortcuts::{BindShortcutsOptions, GlobalShortcuts, NewShortcut};
use futures_util::StreamExt;
use gtk::{glib, prelude::*};

const TOGGLE_OVERLAY_SHORTCUT_ID: &str = "toggle-overlay";
const TOGGLE_OVERLAY_DESCRIPTION: &str = "Show or hide the Quadra overlay";
const TOGGLE_OVERLAY_PREFERRED_TRIGGER: &str = "CTRL+SHIFT+o";

pub fn listen_for_toggle_overlay_shortcut(
    parent: &impl IsA<gtk::Native>,
    on_toggle: impl Fn() + 'static,
) {
    let parent = parent.clone().upcast::<gtk::Native>();
    glib::spawn_future_local(async move {
        if let Err(error) = toggle_on_every_activation(&parent, on_toggle).await {
            eprintln!("Failed to register the overlay shortcut: {error}");
        }
    });
}

async fn toggle_on_every_activation(
    parent: &gtk::Native,
    on_toggle: impl Fn(),
) -> ashpd::Result<()> {
    let portal = GlobalShortcuts::new().await?;
    let session = portal
        .create_session(CreateSessionOptions::default())
        .await?;
    let parent = WindowIdentifier::from_native(parent).await;
    let toggle_overlay = NewShortcut::new(TOGGLE_OVERLAY_SHORTCUT_ID, TOGGLE_OVERLAY_DESCRIPTION)
        .preferred_trigger(TOGGLE_OVERLAY_PREFERRED_TRIGGER);

    let mut activations = portal.receive_activated().await?;
    portal
        .bind_shortcuts(
            &session,
            &[toggle_overlay],
            parent.as_ref(),
            BindShortcutsOptions::default(),
        )
        .await?
        .response()?;

    while let Some(activation) = activations.next().await {
        if activation.shortcut_id() == TOGGLE_OVERLAY_SHORTCUT_ID {
            on_toggle();
        }
    }
    Ok(())
}
