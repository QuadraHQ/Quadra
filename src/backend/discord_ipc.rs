use discord_rich_presence::activity::{Activity, Assets, Button, Timestamps};
use discord_rich_presence::{DiscordIpc, DiscordIpcClient};
use std::error::Error;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::Duration;

const RECONNECT_DELAY: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pub url: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    pub label: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RichPresence {
    pub details: String,
    pub state: Option<String>,
    pub started_at: Option<i64>,
    pub large_image: Image,
    pub small_image: Option<Image>,
    pub link: Option<Link>,
}

pub struct DiscordClient {
    presences: mpsc::Sender<Option<RichPresence>>,
}

impl DiscordClient {
    pub fn start(client_id: &'static str) -> Self {
        let (presences, wanted_presences) = mpsc::channel();
        std::thread::spawn(move || keep_presence_shown(client_id, &wanted_presences));
        Self { presences }
    }

    pub fn show(&self, presence: Option<RichPresence>) {
        let _ = self.presences.send(presence);
    }
}

fn keep_presence_shown(client_id: &str, wanted_presences: &mpsc::Receiver<Option<RichPresence>>) {
    let mut wanted_presence = None;
    let mut connection = None;
    let mut discord_is_up_to_date = true;

    loop {
        let next_presence = if discord_is_up_to_date {
            wanted_presences
                .recv()
                .map_err(|_| RecvTimeoutError::Disconnected)
        } else {
            wanted_presences.recv_timeout(RECONNECT_DELAY)
        };

        match next_presence {
            Ok(presence) => wanted_presence = presence,
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        while let Ok(newer_presence) = wanted_presences.try_recv() {
            wanted_presence = newer_presence;
        }

        discord_is_up_to_date = show(client_id, &mut connection, wanted_presence.as_ref());
    }
}

fn show(
    client_id: &str,
    connection: &mut Option<DiscordIpcClient>,
    presence: Option<&RichPresence>,
) -> bool {
    let Some(presence) = presence else {
        if let Some(mut client) = connection.take() {
            let _ = client.clear_activity().and_then(|()| client.recv());
            let _ = client.close();
        }
        return true;
    };

    if connection.is_none() {
        *connection = connect(client_id);
    }
    let Some(client) = connection else {
        return false;
    };

    match set_presence(client, presence) {
        Ok(()) => true,
        Err(error) => {
            eprintln!("Couldn't show the Discord activity: {error}");
            let _ = client.close();
            *connection = None;
            false
        }
    }
}

fn connect(client_id: &str) -> Option<DiscordIpcClient> {
    let mut client = DiscordIpcClient::new(client_id);
    client.connect().ok().map(|()| client)
}

fn set_presence(
    client: &mut DiscordIpcClient,
    presence: &RichPresence,
) -> Result<(), Box<dyn Error>> {
    client.set_activity(as_activity(presence))?;

    let (_, reply) = client.recv()?;
    if reply["evt"] == "ERROR" {
        let message = reply["data"]["message"].as_str().unwrap_or("unknown error");
        return Err(format!("Discord refused it: {message}").into());
    }
    Ok(())
}

fn as_activity(presence: &RichPresence) -> Activity<'_> {
    let mut assets = Assets::new()
        .large_image(&presence.large_image.url)
        .large_text(&presence.large_image.text);
    if let Some(small_image) = &presence.small_image {
        assets = assets
            .small_image(&small_image.url)
            .small_text(&small_image.text);
    }

    let mut activity = Activity::new().details(&presence.details).assets(assets);
    if let Some(state) = &presence.state {
        activity = activity.state(state);
    }
    if let Some(started_at) = presence.started_at {
        activity = activity.timestamps(Timestamps::new().start(started_at));
    }
    if let Some(link) = &presence.link {
        activity = activity.buttons(vec![Button::new(&link.label, &link.url)]);
    }
    activity
}
