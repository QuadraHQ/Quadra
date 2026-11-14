mod release;

use super::background::{Progress, run_quietly};
use super::download::{
    checksum_listed_for, download_file, remove_directory_if_present, replace_directory,
    sha256_of_file,
};
use super::paths;
use super::req_agent::get_text;
use super::updater::version::Version;
use crate::state::AppState;
use anyhow::{Context, bail, ensure};
use flate2::read::GzDecoder;
use release::RosemaryRelease;
use std::cell::RefCell;
use std::fs::{self, File};
use std::path::PathBuf;

pub const VERSION: &str = "v0.5.0-quadra.2";

const SUPPORTED_ARCHITECTURE: &str = "x86_64";
const ARCHIVE_NAME: &str = "rosemary-steam-tool-0.5.0-quadra.2-x86_64.tar.gz";
const ARCHIVE_URL: &str = "https://github.com/QuadraHQ/rosemary-fork/releases/download/v0.5.0-quadra.2/rosemary-steam-tool-0.5.0-quadra.2-x86_64.tar.gz";
const ARCHIVE_SHA256: &str = "cacf13c3d14835078c00e43f754e1c85bd01ec5466fcb0b56db11e790e859676";
const EXECUTABLE_INSIDE_ARCHIVE: &str = "Rosemary/bin/rosemary";

thread_local! {
    static AVAILABLE_UPDATE: RefCell<Option<RosemaryRelease>> = const { RefCell::new(None) };
}

struct Archive<'a> {
    version: &'a str,
    name: &'a str,
    url: &'a str,
    sha256: &'a str,
}

pub fn executable() -> PathBuf {
    paths::quadra_rosemary_directory(&version_to_run()).join(EXECUTABLE_INSIDE_ARCHIVE)
}

pub fn installed_version() -> Option<String> {
    fs::read_dir(paths::quadra_rosemary_versions_directory())
        .ok()?
        .flatten()
        .filter(|entry| entry.path().join(EXECUTABLE_INSIDE_ARCHIVE).is_file())
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            Some((Version::parse(&name)?, name))
        })
        .max_by(|(a, _), (b, _)| a.cmp(b))
        .map(|(_, name)| name)
}

fn version_to_run() -> String {
    let bundled = Version::parse(VERSION);
    installed_version()
        .filter(|installed| Version::parse(installed) >= bundled)
        .unwrap_or_else(|| VERSION.to_owned())
}

pub fn install(progress: &Progress) -> anyhow::Result<()> {
    install_archive(
        &Archive {
            version: VERSION,
            name: ARCHIVE_NAME,
            url: ARCHIVE_URL,
            sha256: ARCHIVE_SHA256,
        },
        progress,
    )
}

pub fn look_for_update(state: &AppState) {
    let Some(installed) = installed_version() else {
        return;
    };

    let state = state.clone();
    run_quietly(
        move || release::newest_after(&installed),
        move |newest| match newest {
            Ok(newest) => {
                state.set_newer_quadra_rosemary(
                    newest
                        .as_ref()
                        .map(|release| release.version.clone())
                        .unwrap_or_default(),
                );
                AVAILABLE_UPDATE.replace(newest);
            }
            Err(error) => eprintln!("Couldn't check for a newer Rosemary: {error:#}"),
        },
    );
}

pub fn available_update() -> Option<RosemaryRelease> {
    AVAILABLE_UPDATE.with_borrow(Clone::clone)
}

pub fn forget_installed_update(state: &AppState) {
    let newer = state.newer_quadra_rosemary();
    if !newer.is_empty() && installed_version().as_deref() == Some(newer.as_str()) {
        state.set_newer_quadra_rosemary("");
        AVAILABLE_UPDATE.replace(None);
    }
}

pub fn update(release: &RosemaryRelease, progress: &Progress) -> anyhow::Result<()> {
    progress.step(format!(
        "Finding the checksum of Rosemary {}",
        release.version
    ));
    let checksums = get_text(&release.checksums_url)?;
    let sha256 = checksum_listed_for(&checksums, &release.archive_name).with_context(|| {
        format!(
            "Rosemary {}'s checksums don't list {}, so it wasn't installed.",
            release.version, release.archive_name
        )
    })?;

    install_archive(
        &Archive {
            version: &release.version,
            name: &release.archive_name,
            url: &release.archive_url,
            sha256: &sha256,
        },
        progress,
    )?;
    remove_versions_other_than(&release.version);
    Ok(())
}

fn remove_versions_other_than(kept_version: &str) {
    let Ok(entries) = fs::read_dir(paths::quadra_rosemary_versions_directory()) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let is_an_old_version = name
            .to_str()
            .is_some_and(|name| name != kept_version && Version::parse(name).is_some());
        if is_an_old_version && let Err(error) = remove_directory_if_present(&entry.path()) {
            eprintln!(
                "Couldn't remove the old Rosemary at {}: {error:#}",
                entry.path().display()
            );
        }
    }
}

fn install_archive(archive: &Archive, progress: &Progress) -> anyhow::Result<()> {
    if std::env::consts::ARCH != SUPPORTED_ARCHITECTURE {
        bail!(
            "Quadra's Rosemary is only built for {SUPPORTED_ARCHITECTURE} computers. \
             Install Rosemary yourself or with Butter instead."
        );
    }
    let version = archive.version;

    let downloaded = paths::downloads_directory().join(archive.name);
    progress.step(format!("Downloading Rosemary {version}"));
    download_file(archive.url, &downloaded, progress)?;

    progress.step("Verifying Rosemary");
    let checksum = sha256_of_file(&downloaded)?;
    if checksum != archive.sha256 {
        let _ = fs::remove_file(&downloaded);
        bail!(
            "The Rosemary download is damaged: its SHA-256 is {checksum}, but {} was expected.",
            archive.sha256
        );
    }

    progress.step(format!("Unpacking Rosemary {version}"));
    let destination = paths::quadra_rosemary_directory(version);
    let unpacking = destination.with_file_name(format!(".{version}.unpacking"));
    remove_directory_if_present(&unpacking)?;
    fs::create_dir_all(&unpacking)?;

    let mut tar = tar::Archive::new(GzDecoder::new(File::open(&downloaded)?));
    tar.set_preserve_permissions(true);
    for entry in tar.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.into_owned();
        let stayed_inside = entry
            .unpack_in(&unpacking)
            .with_context(|| format!("Couldn't unpack {}", path.display()))?;
        ensure!(
            stayed_inside,
            "Refused to unpack {}: it points outside the archive.",
            path.display()
        );
    }

    ensure!(
        unpacking.join(EXECUTABLE_INSIDE_ARCHIVE).is_file(),
        "The Rosemary download has no {EXECUTABLE_INSIDE_ARCHIVE}."
    );
    replace_directory(&unpacking, &destination)?;
    let _ = fs::remove_file(&downloaded);

    Ok(())
}
