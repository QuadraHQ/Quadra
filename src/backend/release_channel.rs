use std::fs;
use std::path::{Path, PathBuf};

const USERS_INSIDE_PREFIX: &str = "root/Users";
const PLAYER_CHANNEL_INSIDE_USER: &str = "Library/Preferences/com.roblox.RobloxPlayerChannel.plist";
const ROBLOX_SITE_KEY: &str = "<key>www.roblox.com</key>";
const PRODUCTION_CHANNEL: &str = "";
const PRODUCTION_CHANNEL_NAME: &str = "live";
const PRODUCTION_PREFERENCES: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>www.roblox.com</key>
	<string></string>
</dict>
</plist>
"#;

pub fn keep_player_on_production(prefix_directory: &Path) {
    for preferences in player_channel_files(prefix_directory) {
        let Ok(contents) = fs::read(&preferences) else {
            continue;
        };
        let channel = saved_channel(&contents);
        if channel.as_deref().is_some_and(is_production) {
            continue;
        }

        match fs::write(&preferences, PRODUCTION_PREFERENCES) {
            Ok(()) => eprintln!(
                "Roblox had saved the {} channel, which Quadra doesn't install, so it was set back \
                 to production to stop Roblox's installer from running",
                channel.as_deref().unwrap_or("unreadable")
            ),
            Err(error) => eprintln!(
                "Couldn't set Roblox's channel back to production in {}: {error}",
                preferences.display()
            ),
        }
    }
}

fn is_production(channel: &str) -> bool {
    channel == PRODUCTION_CHANNEL || channel.eq_ignore_ascii_case(PRODUCTION_CHANNEL_NAME)
}

fn player_channel_files(prefix_directory: &Path) -> Vec<PathBuf> {
    fs::read_dir(prefix_directory.join(USERS_INSIDE_PREFIX))
        .into_iter()
        .flatten()
        .flatten()
        .map(|user| user.path().join(PLAYER_CHANNEL_INSIDE_USER))
        .filter(|preferences| preferences.is_file())
        .collect()
}

fn saved_channel(preferences: &[u8]) -> Option<String> {
    let preferences = std::str::from_utf8(preferences).ok()?;
    let (_, after_key) = preferences.split_once(ROBLOX_SITE_KEY)?;
    let value = after_key.trim_start();
    if value.starts_with("<string/>") {
        return Some(String::new());
    }
    let (channel, _) = value.strip_prefix("<string>")?.split_once("</string>")?;
    Some(channel.trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn preferences_with(value: &str) -> String {
        PRODUCTION_PREFERENCES.replace("<string></string>", value)
    }

    #[test]
    fn reads_the_saved_channel() {
        assert_eq!(
            saved_channel(preferences_with("<string>zbisect002</string>").as_bytes()).as_deref(),
            Some("zbisect002")
        );
    }

    #[test]
    fn reads_production_in_both_spellings() {
        assert_eq!(
            saved_channel(PRODUCTION_PREFERENCES.as_bytes()).as_deref(),
            Some(PRODUCTION_CHANNEL)
        );
        assert_eq!(
            saved_channel(preferences_with("<string/>").as_bytes()).as_deref(),
            Some(PRODUCTION_CHANNEL)
        );
    }

    #[test]
    fn cannot_read_binary_or_other_sites() {
        assert_eq!(saved_channel(b"bplist00\x01\x02"), None);
        assert_eq!(
            saved_channel(
                PRODUCTION_PREFERENCES
                    .replace("www.roblox.com", "www.example.com")
                    .as_bytes()
            ),
            None
        );
    }

    #[test]
    fn only_production_counts_as_production() {
        for production in ["", "live", "LIVE"] {
            assert!(is_production(production), "{production}");
        }
        for test_channel in [
            "zbisect002",
            "zcanary",
            "zintegration",
            "zflag",
            "znext",
            "beta",
        ] {
            assert!(!is_production(test_channel), "{test_channel}");
        }
    }

    #[test]
    fn sets_every_test_channel_back_to_production() {
        for test_channel in ["zbisect002", "zcanary", "zintegration", "znext"] {
            let prefix = std::env::temp_dir().join(format!(
                "quadra-channel-{test_channel}-{}",
                std::process::id()
            ));
            let preferences = prefix
                .join(USERS_INSIDE_PREFIX)
                .join("someone")
                .join(PLAYER_CHANNEL_INSIDE_USER);
            fs::create_dir_all(preferences.parent().unwrap()).unwrap();
            fs::write(
                &preferences,
                preferences_with(&format!("<string>{test_channel}</string>")),
            )
            .unwrap();

            keep_player_on_production(&prefix);

            assert_eq!(
                fs::read_to_string(&preferences).unwrap(),
                PRODUCTION_PREFERENCES,
                "{test_channel}"
            );
            fs::remove_dir_all(&prefix).unwrap();
        }
    }

    #[test]
    fn leaves_production_alone() {
        let prefix =
            std::env::temp_dir().join(format!("quadra-channel-live-{}", std::process::id()));
        let preferences = prefix
            .join(USERS_INSIDE_PREFIX)
            .join("someone")
            .join(PLAYER_CHANNEL_INSIDE_USER);
        fs::create_dir_all(preferences.parent().unwrap()).unwrap();
        let live = preferences_with("<string>LIVE</string>");
        fs::write(&preferences, &live).unwrap();

        keep_player_on_production(&prefix);

        assert_eq!(fs::read_to_string(&preferences).unwrap(), live);
        fs::remove_dir_all(&prefix).unwrap();
    }

    #[test]
    fn sets_a_custom_channel_back_to_production() {
        let prefix = std::env::temp_dir().join(format!("quadra-channel-{}", std::process::id()));
        let preferences = prefix
            .join(USERS_INSIDE_PREFIX)
            .join("someone")
            .join(PLAYER_CHANNEL_INSIDE_USER);
        fs::create_dir_all(preferences.parent().unwrap()).unwrap();
        fs::write(
            &preferences,
            preferences_with("<string>zbisect002</string>"),
        )
        .unwrap();

        keep_player_on_production(&prefix);

        assert_eq!(
            fs::read_to_string(&preferences).unwrap(),
            PRODUCTION_PREFERENCES
        );
        fs::remove_dir_all(&prefix).unwrap();
    }
}
