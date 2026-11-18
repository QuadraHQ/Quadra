use super::background::Progress;
use super::regions::{self, PickedServer};

pub enum Destination {
    RobloxApp,
    Link(String),
    AnyServer {
        place_id: u64,
    },
    Server {
        place_id: u64,
        job_id: String,
    },
    ServerInRegion {
        place_id: u64,
        region_key: String,
        region_label: String,
        avoided_job_ids: Vec<String>,
    },
}

pub struct Launch {
    pub uri: Option<String>,
    pub picked_server: Option<PickedServer>,
}

impl Destination {
    pub fn resolve(self, progress: &Progress) -> Launch {
        match self {
            Self::RobloxApp => Launch {
                uri: None,
                picked_server: None,
            },
            Self::Link(link) => Launch {
                uri: Some(link),
                picked_server: None,
            },
            Self::AnyServer { place_id } => any_server_in(place_id),
            Self::Server { place_id, job_id } => Launch {
                uri: Some(game_uri(place_id, Some(&job_id))),
                picked_server: None,
            },
            Self::ServerInRegion {
                place_id,
                region_key,
                region_label,
                avoided_job_ids,
            } => {
                progress.step(format!("Looking for a server in {region_label}"));
                match regions::pick_server(place_id, &region_key, &avoided_job_ids) {
                    Ok(Some(picked_server)) => Launch {
                        uri: Some(game_uri(place_id, Some(&picked_server.job_id))),
                        picked_server: Some(picked_server),
                    },
                    Ok(None) => {
                        eprintln!("No server found in {region_label}, Roblox will choose one");
                        any_server_in(place_id)
                    }
                    Err(error) => {
                        eprintln!("{error:#}, Roblox will choose a server");
                        any_server_in(place_id)
                    }
                }
            }
        }
    }
}

fn any_server_in(place_id: u64) -> Launch {
    Launch {
        uri: Some(game_uri(place_id, None)),
        picked_server: None,
    }
}

fn game_uri(place_id: u64, job_id: Option<&str>) -> String {
    let mut uri = format!("roblox://experiences/start?placeId={place_id}");
    if let Some(job_id) = job_id.filter(|job_id| is_job_id(job_id)) {
        uri.push_str(&format!("&gameInstanceId={job_id}"));
    }
    uri
}

fn is_job_id(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|character| character.is_ascii_hexdigit() || character == '-')
}
