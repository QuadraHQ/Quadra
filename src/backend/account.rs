use super::prefix;
use crate::state::AppState;
use std::fs;
use std::path::{Path, PathBuf};

const USERS_INSIDE_PREFIX: &str = "root/Users";
const COOKIES_INSIDE_USER: &str = "Library/Cookies";
const COOKIE_FILE_PREFIX: &str = "com.roblox.";
const COOKIE_FILE_SUFFIX: &str = ".cookies.tsv";
const SESSION_COOKIE: &str = "ROBLOSECURITY";

pub fn refresh(state: &AppState) {
    let signed_in = prefix::directory_for(&state.prefix())
        .is_some_and(|prefix_directory| is_signed_in(&prefix_directory));
    state.set_signed_in(signed_in);
}

pub fn sign_out(state: &AppState) {
    if state.running() {
        state.report_error("Close Roblox before signing out, or it would save the session again.");
        return;
    }
    let Some(prefix_directory) = prefix::directory_for(&state.prefix()) else {
        state.report_error("Quadra can only sign out of its own prefix.");
        return;
    };

    let failed: Vec<String> = cookie_files(&prefix_directory)
        .into_iter()
        .filter_map(|file| {
            fs::remove_file(&file)
                .err()
                .map(|error| format!("{}: {error}", file.display()))
        })
        .collect();
    if !failed.is_empty() {
        state.report_error(&format!("Couldn't sign out:\n{}", failed.join("\n")));
    }
    refresh(state);
}

fn is_signed_in(prefix_directory: &Path) -> bool {
    cookie_files(prefix_directory)
        .iter()
        .filter_map(|file| fs::read_to_string(file).ok())
        .any(|cookies| cookies.contains(SESSION_COOKIE))
}

fn cookie_files(prefix_directory: &Path) -> Vec<PathBuf> {
    let users = prefix_directory.join(USERS_INSIDE_PREFIX);
    let mut files: Vec<PathBuf> = fs::read_dir(users)
        .into_iter()
        .flatten()
        .flatten()
        .flat_map(|user| {
            fs::read_dir(user.path().join(COOKIES_INSIDE_USER))
                .into_iter()
                .flatten()
                .flatten()
        })
        .map(|cookies| cookies.path())
        .filter(|file| is_roblox_cookie_file(file))
        .collect();
    files.sort();
    files
}

fn is_roblox_cookie_file(file: &Path) -> bool {
    file.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            name.starts_with(COOKIE_FILE_PREFIX) && name.ends_with(COOKIE_FILE_SUFFIX)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_only_roblox_cookie_files_of_every_user() {
        let root = std::env::temp_dir().join(format!("quadra-account-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        for user in ["a", "b"] {
            let cookies = root
                .join(USERS_INSIDE_PREFIX)
                .join(user)
                .join(COOKIES_INSIDE_USER);
            fs::create_dir_all(&cookies).unwrap();
            fs::write(
                cookies.join("com.roblox.RobloxPlayer.cookies.tsv"),
                "x ROBLOSECURITY y",
            )
            .unwrap();
            fs::write(cookies.join("other.cookies.tsv"), "").unwrap();
        }

        assert_eq!(cookie_files(&root).len(), 2);
        assert!(is_signed_in(&root));
        for file in cookie_files(&root) {
            fs::remove_file(file).unwrap();
        }
        assert!(!is_signed_in(&root));
        let _ = fs::remove_dir_all(&root);
    }
}
