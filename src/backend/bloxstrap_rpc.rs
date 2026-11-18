use super::discord_ipc::{Image, RichPresence};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

pub const MARKER: &str = "[BloxstrapRPC] ";
const SET_RICH_PRESENCE: &str = "SetRichPresence";
const RESET_TEXT: &str = "<reset>";
const ASSET_ID_PREFIX: &str = "rbxassetid://";
const LONGEST_TEXT: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Setting<T> {
    #[default]
    Default,
    Erased,
    Value(T),
}

impl<T> Setting<T> {
    fn resolve<U>(&self, default: Option<U>, from: impl FnOnce(&T) -> Option<U>) -> Option<U> {
        match self {
            Self::Default => default,
            Self::Erased => None,
            Self::Value(value) => from(value),
        }
    }

    fn replace_with(&mut self, update: Option<Self>) {
        if let Some(update) = update {
            *self = update;
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ImageSetting {
    asset_id: Setting<u64>,
    hover_text: Setting<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ImageUpdate {
    asset_id: Option<Setting<u64>>,
    hover_text: Option<Setting<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PresenceUpdate {
    details: Option<Setting<String>>,
    state: Option<Setting<String>>,
    started_at: Option<Setting<i64>>,
    ended_at: Option<Setting<i64>>,
    small_image: ImageUpdate,
    large_image: ImageUpdate,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Overrides {
    details: Setting<String>,
    state: Setting<String>,
    started_at: Setting<i64>,
    ended_at: Setting<i64>,
    small_image: ImageSetting,
    large_image: ImageSetting,
}

impl Overrides {
    pub fn apply(&mut self, update: PresenceUpdate) {
        self.details.replace_with(update.details);
        self.state.replace_with(update.state);
        self.started_at.replace_with(update.started_at);
        self.ended_at.replace_with(update.ended_at);
        self.small_image.apply(update.small_image);
        self.large_image.apply(update.large_image);
    }

    pub fn asset_ids(&self) -> impl Iterator<Item = u64> {
        [&self.small_image, &self.large_image]
            .into_iter()
            .filter_map(|image| match image.asset_id {
                Setting::Value(asset_id) => Some(asset_id),
                _ => None,
            })
    }

    pub fn over(&self, presence: RichPresence, asset_urls: &HashMap<u64, String>) -> RichPresence {
        RichPresence {
            details: self
                .details
                .resolve(Some(presence.details), |text| Some(text.clone()))
                .unwrap_or_default(),
            state: self
                .state
                .resolve(presence.state, |text| Some(text.clone())),
            started_at: self
                .started_at
                .resolve(presence.started_at, |seconds| Some(*seconds)),
            ended_at: self
                .ended_at
                .resolve(presence.ended_at, |seconds| Some(*seconds)),
            large_image: self.large_image.over(presence.large_image, asset_urls),
            small_image: self.small_image.over(presence.small_image, asset_urls),
            link: presence.link,
        }
    }
}

impl ImageSetting {
    fn apply(&mut self, update: ImageUpdate) {
        self.asset_id.replace_with(update.asset_id);
        self.hover_text.replace_with(update.hover_text);
    }

    fn over(&self, default: Option<Image>, asset_urls: &HashMap<u64, String>) -> Option<Image> {
        let image = self.asset_id.resolve(default.clone(), |asset_id| {
            let url = asset_urls.get(asset_id)?;
            let text = default.as_ref().map(|image| image.text.clone());
            Some(Image {
                url: url.clone(),
                text: text.unwrap_or_default(),
            })
        });
        let image = image.or(default.filter(|_| self.asset_id_is_loading(asset_urls)))?;
        let text = self
            .hover_text
            .resolve(Some(image.text.clone()), |text| Some(text.clone()))
            .unwrap_or_default();
        Some(Image { text, ..image })
    }

    fn asset_id_is_loading(&self, asset_urls: &HashMap<u64, String>) -> bool {
        matches!(&self.asset_id, Setting::Value(asset_id) if !asset_urls.contains_key(asset_id))
    }
}

#[derive(Deserialize)]
struct Message {
    command: String,
    #[serde(default)]
    data: Value,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct RichPresenceData {
    details: Option<String>,
    state: Option<String>,
    time_start: Option<f64>,
    time_end: Option<f64>,
    small_image: Option<ImageData>,
    large_image: Option<ImageData>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct ImageData {
    asset_id: Option<Value>,
    hover_text: Option<String>,
    clear: bool,
    reset: bool,
}

pub fn presence_update_in(line: &str) -> Option<PresenceUpdate> {
    let (_, payload) = line.split_once(MARKER)?;
    let message: Message = serde_json::from_str(payload.trim()).ok()?;
    if message.command != SET_RICH_PRESENCE {
        return None;
    }
    let data: RichPresenceData = serde_json::from_value(message.data).ok()?;
    Some(PresenceUpdate {
        details: data.details.and_then(text_setting),
        state: data.state.and_then(text_setting),
        started_at: data.time_start.map(time_setting),
        ended_at: data.time_end.map(time_setting),
        small_image: data.small_image.map(image_update).unwrap_or_default(),
        large_image: data.large_image.map(image_update).unwrap_or_default(),
    })
}

fn text_setting(text: String) -> Option<Setting<String>> {
    if text == RESET_TEXT {
        Some(Setting::Default)
    } else if text.is_empty() {
        Some(Setting::Erased)
    } else if text.chars().count() > LONGEST_TEXT {
        None
    } else {
        Some(Setting::Value(text))
    }
}

fn time_setting(seconds: f64) -> Setting<i64> {
    if seconds == 0.0 {
        Setting::Erased
    } else {
        Setting::Value(seconds as i64)
    }
}

fn image_update(data: ImageData) -> ImageUpdate {
    if data.reset {
        return ImageUpdate {
            asset_id: Some(Setting::Default),
            hover_text: Some(Setting::Default),
        };
    }
    let asset_id = if data.clear {
        Some(Setting::Erased)
    } else {
        data.asset_id.and_then(asset_id_in).map(Setting::Value)
    };
    ImageUpdate {
        asset_id,
        hover_text: data.hover_text.and_then(text_setting),
    }
}

fn asset_id_in(value: Value) -> Option<u64> {
    match value {
        Value::Number(number) => number.as_f64().map(|asset_id| asset_id as u64),
        Value::String(text) => text
            .strip_prefix(ASSET_ID_PREFIX)
            .unwrap_or(&text)
            .parse()
            .ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(json: &str) -> String {
        format!(
            "2026-11-17T01:13:02.661Z,0.66,e8560000,6,Info [FLog::Output] [BloxstrapRPC] {json}"
        )
    }

    fn update(json: &str) -> PresenceUpdate {
        presence_update_in(&line(json)).expect("a rich presence update")
    }

    fn game() -> RichPresence {
        RichPresence {
            details: "Game".to_owned(),
            state: Some("by Someone".to_owned()),
            started_at: Some(100),
            ended_at: None,
            large_image: Some(Image {
                url: "game-icon".to_owned(),
                text: "Game".to_owned(),
            }),
            small_image: Some(Image {
                url: "quadra".to_owned(),
                text: "Quadra".to_owned(),
            }),
            link: None,
        }
    }

    fn merged(updates: &[&str], asset_urls: &HashMap<u64, String>) -> RichPresence {
        let mut overrides = Overrides::default();
        for json in updates {
            overrides.apply(update(json));
        }
        overrides.over(game(), asset_urls)
    }

    #[test]
    fn ignores_lines_that_are_not_rich_presence_commands() {
        assert_eq!(presence_update_in("[FLog::Output] hello"), None);
        assert_eq!(presence_update_in(&line("{not json")), None);
        assert_eq!(
            presence_update_in(&line(r#"{"command":"SetLaunchData","data":"x"}"#)),
            None
        );
    }

    #[test]
    fn sets_details_and_state_and_keeps_what_was_not_sent() {
        let presence = merged(
            &[
                r#"{"command":"SetRichPresence","data":{"details":"Round 3","state":"Alive"}}"#,
                r#"{"command":"SetRichPresence","data":{"state":"Dead"}}"#,
            ],
            &HashMap::new(),
        );
        assert_eq!(presence.details, "Round 3");
        assert_eq!(presence.state.as_deref(), Some("Dead"));
        assert_eq!(presence.started_at, Some(100));
    }

    #[test]
    fn erases_with_an_empty_value_and_restores_with_reset() {
        let erased = merged(
            &[r#"{"command":"SetRichPresence","data":{"state":"","timeStart":0}}"#],
            &HashMap::new(),
        );
        assert_eq!(erased.state, None);
        assert_eq!(erased.started_at, None);

        let restored = merged(
            &[
                r#"{"command":"SetRichPresence","data":{"state":"","details":"x"}}"#,
                r#"{"command":"SetRichPresence","data":{"state":"<reset>","details":"<reset>"}}"#,
            ],
            &HashMap::new(),
        );
        assert_eq!(restored.state.as_deref(), Some("by Someone"));
        assert_eq!(restored.details, "Game");
    }

    #[test]
    fn ignores_text_longer_than_discord_allows() {
        let long = "x".repeat(129);
        let presence = merged(
            &[&format!(
                r#"{{"command":"SetRichPresence","data":{{"details":"{long}"}}}}"#
            )],
            &HashMap::new(),
        );
        assert_eq!(presence.details, "Game");
    }

    #[test]
    fn replaces_an_image_once_its_url_is_known_and_keeps_the_default_until_then() {
        let json = r#"{"command":"SetRichPresence","data":{"largeImage":{"assetId":"rbxassetid://42","hoverText":"Lobby"}}}"#;
        let loading = merged(&[json], &HashMap::new());
        assert_eq!(loading.large_image.as_ref().unwrap().url, "game-icon");
        assert_eq!(loading.large_image.as_ref().unwrap().text, "Lobby");

        let urls = HashMap::from([(42, "https://tr.rbxcdn.com/42".to_owned())]);
        let loaded = merged(&[json], &urls);
        assert_eq!(
            loaded.large_image,
            Some(Image {
                url: "https://tr.rbxcdn.com/42".to_owned(),
                text: "Lobby".to_owned()
            })
        );
    }

    #[test]
    fn clears_and_resets_images() {
        let cleared = merged(
            &[r#"{"command":"SetRichPresence","data":{"smallImage":{"clear":true}}}"#],
            &HashMap::new(),
        );
        assert_eq!(cleared.small_image, None);
        assert!(cleared.large_image.is_some());

        let reset = merged(
            &[
                r#"{"command":"SetRichPresence","data":{"smallImage":{"clear":true}}}"#,
                r#"{"command":"SetRichPresence","data":{"smallImage":{"reset":true}}}"#,
            ],
            &HashMap::new(),
        );
        assert_eq!(reset.small_image, game().small_image);
    }

    #[test]
    fn reads_asset_ids_given_as_numbers_or_rbxassetid_strings() {
        let overrides = {
            let mut overrides = Overrides::default();
            overrides.apply(update(
                r#"{"command":"SetRichPresence","data":{"largeImage":{"assetId":10630555127},"smallImage":{"assetId":"rbxassetid://13409122839"}}}"#,
            ));
            overrides
        };
        let mut ids: Vec<u64> = overrides.asset_ids().collect();
        ids.sort();
        assert_eq!(ids, [10630555127, 13409122839]);
    }
}
