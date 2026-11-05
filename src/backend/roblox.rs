use super::background::Progress;
use super::download::{
    download_file, remove_directory_if_present, replace_directory, stop_others_from_writing,
};
use super::paths;
use anyhow::{Context, ensure};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::path::PathBuf;

const LATEST_RELEASE_URL: &str = "https://clientsettingscdn.roblox.com/v2/client-version/MacPlayer";
const DOWNLOAD_BASE_URL: &str = "https://setup.rbxcdn.com/mac";
const APP_NAME: &str = "RobloxPlayer.app";
const EXECUTABLE_INSIDE_APP: &str = "Contents/MacOS/RobloxPlayer";
const INSTALLED_RELEASE_FILE: &str = "release.toml";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RobloxRelease {
    pub version: String,
    pub client_version_upload: String,
}

pub fn app_path() -> PathBuf {
    paths::roblox_directory().join(APP_NAME)
}

pub fn installed_release() -> Option<RobloxRelease> {
    if !app_path().join(EXECUTABLE_INSIDE_APP).is_file() {
        return None;
    }
    let release = fs::read_to_string(installed_release_path()).ok()?;
    toml::from_str(&release).ok()
}

pub fn latest_release() -> anyhow::Result<RobloxRelease> {
    ureq::get(LATEST_RELEASE_URL)
        .call()
        .and_then(|mut response| response.body_mut().read_json())
        .context("Couldn't check for the latest Roblox version")
}

pub fn install(release: &RobloxRelease, progress: &Progress) -> anyhow::Result<()> {
    let archive_name = format!("{}-RobloxPlayer.zip", release.client_version_upload);
    let archive = paths::downloads_directory().join(&archive_name);
    let is_update = installed_release().is_some();
    if is_update {
        progress.step(format!("Downloading the Roblox {} update", release.version));
    } else {
        progress.step(format!("Downloading Roblox {}", release.version));
    }
    download_file(&download_url(&archive_name), &archive, progress)?;

    if is_update {
        progress.step(format!("Updating Roblox to {}", release.version));
    } else {
        progress.step(format!("Installing Roblox {}", release.version));
    }
    let unpacking = paths::roblox_directory().join(".unpacking");
    remove_directory_if_present(&unpacking)?;
    zip::ZipArchive::new(File::open(&archive)?)
        .and_then(|mut zip| zip.extract(&unpacking))
        .context("Couldn't unpack the Roblox download")?;
    stop_others_from_writing(&unpacking)?;

    let unpacked_app = unpacking.join(APP_NAME);
    ensure!(
        unpacked_app.join(EXECUTABLE_INSIDE_APP).is_file(),
        "The Roblox download has no {APP_NAME}."
    );
    replace_directory(&unpacked_app, &app_path())?;
    remove_directory_if_present(&unpacking)?;
    fs::write(installed_release_path(), toml::to_string(release)?)?;
    let _ = fs::remove_file(&archive);

    Ok(())
}

fn download_url(archive_name: &str) -> String {
    match std::env::consts::ARCH {
        "aarch64" => format!("{DOWNLOAD_BASE_URL}/arm64/{archive_name}"),
        _ => format!("{DOWNLOAD_BASE_URL}/{archive_name}"),
    }
}

fn installed_release_path() -> PathBuf {
    paths::roblox_directory().join(INSTALLED_RELEASE_FILE)
}
