use super::req_agent::get_json;
use anyhow::Context;
use serde::Deserialize;

const ICON_IS_READY: &str = "Completed";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameDetails {
    pub name: String,
    pub creator: String,
    pub creator_is_verified: bool,
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
#[serde(rename_all = "camelCase")]
struct Creator {
    name: String,
    #[serde(default)]
    has_verified_badge: bool,
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
    fetch_universe(universe_id)
}

pub fn fetch_universe(universe_id: u64) -> anyhow::Result<GameDetails> {
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
        creator_is_verified: game.creator.has_verified_badge,
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
    Ok(completed_image_url(icons))
}

pub fn asset_image_url(asset_id: u64) -> anyhow::Result<Option<String>> {
    let images: Listing<Icon> = get_json(&format!(
        "https://thumbnails.roblox.com/v1/assets?assetIds={asset_id}&returnPolicy=PlaceHolder&size=512x512&format=Png&isCircular=false"
    ))?;
    Ok(completed_image_url(images))
}

fn completed_image_url(images: Listing<Icon>) -> Option<String> {
    images
        .data
        .into_iter()
        .find(|image| image.state == ICON_IS_READY)
        .and_then(|image| image.image_url)
        .filter(|url| !url.is_empty())
}
