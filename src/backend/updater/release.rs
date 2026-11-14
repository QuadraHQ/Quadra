use super::version::Version;
use crate::backend::req_agent::api_agent;
use anyhow::Context;
use serde::Deserialize;

const RELEASES_URL: &str = "https://api.github.com/repos/QuadraHQ/quadra/releases?per_page=30";
const APPIMAGE_SUFFIX: &str = "-x86_64.AppImage";
const CHECKSUMS_NAME: &str = "SHA256SUMS";

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    assets: Vec<GithubAsset>,
}

#[derive(Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppImageUpdate {
    pub version: String,
    pub file_name: String,
    pub url: String,
    pub checksums_url: Option<String>,
}

pub fn newest_after(current: &str) -> anyhow::Result<Option<AppImageUpdate>> {
    let releases: Vec<GithubRelease> = api_agent()
        .get(RELEASES_URL)
        .call()
        .and_then(|mut response| response.body_mut().read_json())
        .context("Couldn't check GitHub for new Quadra releases")?;
    Ok(newest_in(releases, current))
}

pub fn download_checksums(url: &str) -> anyhow::Result<String> {
    api_agent()
        .get(url)
        .call()
        .and_then(|mut response| response.body_mut().read_to_string())
        .context("Couldn't download the release's checksums")
}

fn newest_in(releases: Vec<GithubRelease>, current: &str) -> Option<AppImageUpdate> {
    let current = Version::parse(current)?;
    let include_prereleases = current.is_prerelease();

    releases
        .into_iter()
        .filter(|release| !release.draft && (include_prereleases || !release.prerelease))
        .filter_map(|release| {
            let version = Version::parse(&release.tag_name).filter(|version| *version > current)?;
            Some((version, update_of(release)?))
        })
        .max_by(|(a, _), (b, _)| a.cmp(b))
        .map(|(_, update)| update)
}

fn update_of(release: GithubRelease) -> Option<AppImageUpdate> {
    let checksums_url = release
        .assets
        .iter()
        .find(|asset| asset.name == CHECKSUMS_NAME)
        .map(|asset| asset.browser_download_url.clone());
    let appimage = release
        .assets
        .into_iter()
        .find(|asset| asset.name.ends_with(APPIMAGE_SUFFIX))?;

    Some(AppImageUpdate {
        version: release.tag_name.trim_start_matches('v').to_owned(),
        file_name: appimage.name,
        url: appimage.browser_download_url,
        checksums_url,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str, prerelease: bool, draft: bool, with_appimage: bool) -> GithubRelease {
        let mut assets = vec![GithubAsset {
            name: CHECKSUMS_NAME.to_owned(),
            browser_download_url: format!("https://example/{tag}/SHA256SUMS"),
        }];
        if with_appimage {
            assets.push(GithubAsset {
                name: format!("Quadra-{tag}-x86_64.AppImage"),
                browser_download_url: format!("https://example/{tag}/app"),
            });
        }
        GithubRelease {
            tag_name: tag.to_owned(),
            draft,
            prerelease,
            assets,
        }
    }

    #[test]
    fn alpha_users_follow_prereleases_and_pick_the_highest() {
        let releases = vec![
            release("v0.50.0-alpha.1", true, false, true),
            release("v0.50.0-alpha.3", true, false, true),
            release("v0.50.0-alpha.2", true, false, true),
        ];
        let update = newest_in(releases, "0.50.0-alpha.1").unwrap();
        assert_eq!(update.version, "0.50.0-alpha.3");
        assert_eq!(update.file_name, "Quadra-v0.50.0-alpha.3-x86_64.AppImage");
        assert_eq!(
            update.checksums_url.as_deref(),
            Some("https://example/v0.50.0-alpha.3/SHA256SUMS")
        );
    }

    #[test]
    fn stable_users_skip_prereleases_drafts_and_releases_without_an_appimage() {
        let releases = vec![
            release("v0.51.0-alpha.1", true, false, true),
            release("v0.52.0", false, true, true),
            release("v0.53.0", false, false, false),
            release("v0.49.0", false, false, true),
        ];
        assert_eq!(newest_in(releases, "0.50.0"), None);
    }

    #[test]
    fn nothing_newer_means_no_update() {
        let releases = vec![release("v0.50.0-alpha.2", true, false, true)];
        assert_eq!(newest_in(releases, "0.50.0-alpha.2"), None);
    }

    #[test]
    fn a_stable_release_beats_the_alpha_it_follows() {
        let releases = vec![release("v0.50.0", false, false, true)];
        assert_eq!(
            newest_in(releases, "0.50.0-alpha.9").unwrap().version,
            "0.50.0"
        );
    }
}
