use crate::backend::req_agent::get_json;
use crate::backend::updater::version::Version;
use serde::Deserialize;

const RELEASES_URL: &str =
    "https://api.github.com/repos/QuadraHQ/rosemary-fork/releases?per_page=30";
const QUADRA_BUILD_MARKER: &str = "-quadra.";
const ARCHIVE_PREFIX: &str = "rosemary-steam-tool-";
const ARCHIVE_SUFFIX: &str = "-x86_64.tar.gz";
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
pub struct RosemaryRelease {
    pub version: String,
    pub archive_name: String,
    pub archive_url: String,
    pub checksums_url: String,
}

pub fn newest_after(current: &str) -> anyhow::Result<Option<RosemaryRelease>> {
    let releases: Vec<GithubRelease> = get_json(RELEASES_URL)?;
    Ok(newest_in(releases, current))
}

fn newest_in(releases: Vec<GithubRelease>, current: &str) -> Option<RosemaryRelease> {
    let current = Version::parse(current)?;

    releases
        .into_iter()
        .filter(|release| {
            !release.draft && !release.prerelease && release.tag_name.contains(QUADRA_BUILD_MARKER)
        })
        .filter_map(|release| {
            let version = Version::parse(&release.tag_name).filter(|version| *version > current)?;
            Some((version, release_of(release)?))
        })
        .max_by(|(a, _), (b, _)| a.cmp(b))
        .map(|(_, release)| release)
}

fn release_of(release: GithubRelease) -> Option<RosemaryRelease> {
    let checksums_url = release
        .assets
        .iter()
        .find(|asset| asset.name == CHECKSUMS_NAME)?
        .browser_download_url
        .clone();
    let archive = release.assets.into_iter().find(|asset| {
        asset.name.starts_with(ARCHIVE_PREFIX) && asset.name.ends_with(ARCHIVE_SUFFIX)
    })?;

    Some(RosemaryRelease {
        version: release.tag_name,
        archive_name: archive.name,
        archive_url: archive.browser_download_url,
        checksums_url,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asset(name: &str) -> GithubAsset {
        GithubAsset {
            name: name.to_owned(),
            browser_download_url: format!("https://example/{name}"),
        }
    }

    fn release(tag: &str, prerelease: bool, assets: Vec<GithubAsset>) -> GithubRelease {
        GithubRelease {
            tag_name: tag.to_owned(),
            draft: false,
            prerelease,
            assets,
        }
    }

    fn quadra_build(tag: &str) -> GithubRelease {
        let version = tag.trim_start_matches('v');
        release(
            tag,
            false,
            vec![
                asset(CHECKSUMS_NAME),
                asset(&format!("rosemary-steam-tool-{version}-x86_64.tar.zst")),
                asset(&format!("rosemary-steam-tool-{version}-x86_64.tar.gz")),
            ],
        )
    }

    #[test]
    fn picks_the_highest_quadra_build() {
        let releases = vec![
            quadra_build("v0.5.0-quadra.3"),
            quadra_build("v0.6.0-quadra.1"),
            quadra_build("v0.5.0-quadra.10"),
        ];
        let newest = newest_in(releases, "v0.5.0-quadra.2").unwrap();
        assert_eq!(newest.version, "v0.6.0-quadra.1");
        assert_eq!(
            newest.archive_name,
            "rosemary-steam-tool-0.6.0-quadra.1-x86_64.tar.gz"
        );
        assert_eq!(newest.checksums_url, "https://example/SHA256SUMS");
    }

    #[test]
    fn ignores_upstream_tags_prereleases_and_builds_without_checksums() {
        let mut upstream = quadra_build("v0.6.0");
        upstream.tag_name = "v0.6.0".to_owned();
        let mut prerelease = quadra_build("v0.7.0-quadra.1");
        prerelease.prerelease = true;
        let unverifiable = release(
            "v0.8.0-quadra.1",
            false,
            vec![asset("rosemary-steam-tool-0.8.0-quadra.1-x86_64.tar.gz")],
        );

        let releases = vec![upstream, prerelease, unverifiable];
        assert_eq!(newest_in(releases, "v0.5.0-quadra.2"), None);
    }

    #[test]
    fn nothing_newer_means_no_update() {
        let releases = vec![
            quadra_build("v0.5.0-quadra.1"),
            quadra_build("v0.5.0-quadra.2"),
        ];
        assert_eq!(newest_in(releases, "v0.5.0-quadra.2"), None);
    }
}
