use super::background::Progress;
use super::download::{
    download_file, remove_directory_if_present, replace_directory, sha256_of_file,
};
use super::paths;
use anyhow::{Context, bail, ensure};
use flate2::read::GzDecoder;
use std::fs::{self, File};
use std::path::PathBuf;

pub const VERSION: &str = "v0.5.0-quadra.1";

const SUPPORTED_ARCHITECTURE: &str = "x86_64";
const ARCHIVE_NAME: &str = "rosemary-steam-tool-0.5.0-quadra.1-x86_64.tar.gz";
const ARCHIVE_URL: &str = "https://github.com/QuadraHQ/rosemary-fork/releases/download/v0.5.0-quadra.1/rosemary-steam-tool-0.5.0-quadra.1-x86_64.tar.gz";
const ARCHIVE_SHA256: &str = "51aa929793a486015c180babe91092f5ce170ae404f68bf583c7c7455f61afd3";
const EXECUTABLE_INSIDE_ARCHIVE: &str = "Rosemary/bin/rosemary";

pub fn executable() -> PathBuf {
    paths::quadra_rosemary_directory(VERSION).join(EXECUTABLE_INSIDE_ARCHIVE)
}

pub fn install(progress: &Progress) -> anyhow::Result<()> {
    if std::env::consts::ARCH != SUPPORTED_ARCHITECTURE {
        bail!(
            "Quadra's Rosemary is only built for {SUPPORTED_ARCHITECTURE} computers. \
             Install Rosemary yourself or with Butter instead."
        );
    }

    let archive = paths::downloads_directory().join(ARCHIVE_NAME);
    progress.step(format!("Downloading Rosemary {VERSION}"));
    download_file(ARCHIVE_URL, &archive, progress)?;

    progress.step("Verifying Rosemary");
    let checksum = sha256_of_file(&archive)?;
    ensure!(
        checksum == ARCHIVE_SHA256,
        "The Rosemary download is damaged: its SHA-256 is {checksum}, but {ARCHIVE_SHA256} was expected."
    );

    progress.step("Unpacking Rosemary");
    let destination = paths::quadra_rosemary_directory(VERSION);
    let unpacking = destination.with_file_name(format!(".{VERSION}.unpacking"));
    remove_directory_if_present(&unpacking)?;
    fs::create_dir_all(&unpacking)?;

    let mut tar = tar::Archive::new(GzDecoder::new(File::open(&archive)?));
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
    let _ = fs::remove_file(&archive);

    Ok(())
}
