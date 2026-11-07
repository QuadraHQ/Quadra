const GAME_PAGE_PATH: &str = "/games/";
const URL_PATH_ENDINGS: [char; 3] = ['/', '?', '#'];

pub fn place_id_in(text: &str) -> Option<u64> {
    let text = text.trim();
    let place_id = match text.split_once(GAME_PAGE_PATH) {
        Some((_, after_games)) => after_games.split(URL_PATH_ENDINGS).next()?,
        None => text,
    };
    place_id.parse().ok()
}
