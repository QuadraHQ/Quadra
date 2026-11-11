use crate::utils::get_json;
use anyhow::Context;
use serde::Deserialize;

const ICON_IS_READY: &str = "Completed";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameDetails {
    pub name: String,
    pub creator: String,
    pub icon_url: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Universe {
    universe_id: Option<u64>,
}

#[derive(Deserialize)]
struct Listing<T> {
    data: Vec<T>,
}

#[derive(Deserialize)]
struct Game {
    name: String,
    creator: Creator,
}

#[derive(Deserialize)]
struct Creator {
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Icon {
    state: String,
    image_url: Option<String>,
}

pub fn fetch(place_id: u64) -> anyhow::Result<GameDetails> {
    let universe: Universe = get_json(&format!(
        "https://apis.roblox.com/universes/v1/places/{place_id}/universe"
    ))?;
    let universe_id = universe
        .universe_id
        .with_context(|| format!("Place {place_id} doesn't belong to a game"))?;

    let games: Listing<Game> = get_json(&format!(
        "https://games.roblox.com/v1/games?universeIds={universe_id}"
    ))?;
    let game = games
        .data
        .into_iter()
        .next()
        .with_context(|| format!("Roblox has no details for game {universe_id}"))?;

    Ok(GameDetails {
        name: game.name,
        creator: game.creator.name,
        icon_url: icon_url(universe_id).unwrap_or_else(|error| {
            eprintln!("Couldn't get the icon of game {universe_id}: {error:#}");
            None
        }),
    })
}

fn icon_url(universe_id: u64) -> anyhow::Result<Option<String>> {
    let icons: Listing<Icon> = get_json(&format!(
        "https://thumbnails.roblox.com/v1/games/icons?universeIds={universe_id}&size=512x512&format=Png&isCircular=false"
    ))?;
    Ok(icons
        .data
        .into_iter()
        .find(|icon| icon.state == ICON_IS_READY)
        .and_then(|icon| icon.image_url)
        .filter(|url| !url.is_empty()))
}
