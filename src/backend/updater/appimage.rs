use super::release::{self, AppImageUpdate};
use crate::backend::background::Progress;
use crate::backend::download::{download_file, sha256_of_file};
use anyhow::{Context, bail, ensure};
use std::ffi::CString;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

const EXECUTABLE_MODE: u32 = 0o755;

pub fn replace(
    update: &AppImageUpdate,
    running_appimage: &Path,
    progress: &Progress,
) -> anyhow::Result<()> {
    let directory = running_appimage
        .parent()
        .context("The AppImage has no folder")?;
    ensure!(
        is_writable(directory),
        "Quadra can't write to {}. Move the AppImage to a folder you own, or download the new \
         one from the releases page.",
        directory.display()
    );
    let checksums_url = update
        .checksums_url
        .as_deref()
        .context("This release has no checksums, so Quadra won't install it.")?;

    let staged = staged_path(running_appimage);
    progress.step(format!("Downloading Quadra {}", update.version));
    let installed = download_file(&update.url, &staged, progress)
        .and_then(|()| {
            progress.step("Checking the download");
            release::download_checksums(checksums_url)
        })
        .and_then(|checksums| verify_and_install(&staged, running_appimage, &update.file_name, &checksums));

    if installed.is_err() {
        let _ = fs::remove_file(&staged);
    }
    installed
}

pub fn verify_and_install(
    staged: &Path,
    running_appimage: &Path,
    file_name: &str,
    checksums: &str,
) -> anyhow::Result<()> {
    let expected = expected_checksum(checksums, file_name)
        .with_context(|| format!("The release's checksums don't list {file_name}"))?;
    let actual = sha256_of_file(staged)?;
    if actual != expected {
        bail!("The downloaded {file_name} doesn't match its checksum, so it wasn't installed.");
    }

    fs::set_permissions(staged, fs::Permissions::from_mode(EXECUTABLE_MODE))?;
    fs::rename(staged, running_appimage)
        .with_context(|| format!("Couldn't replace {}", running_appimage.display()))
}

fn expected_checksum(checksums: &str, file_name: &str) -> Option<String> {
    checksums.lines().find_map(|line| {
        let (checksum, name) = line.split_once(char::is_whitespace)?;
        let name = name.trim_start().trim_start_matches('*');
        (name == file_name).then(|| checksum.to_ascii_lowercase())
    })
}

fn staged_path(running_appimage: &Path) -> PathBuf {
    let name = running_appimage
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    running_appimage.with_file_name(format!(".{name}.update"))
}

fn is_writable(directory: &Path) -> bool {
    CString::new(directory.as_os_str().as_bytes())
        .is_ok_and(|path| unsafe { libc::access(path.as_ptr(), libc::W_OK) == 0 })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    struct Sandbox(PathBuf);

    impl Sandbox {
        fn new(name: &str) -> Self {
            let root = std::env::temp_dir().join(format!("quadra-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).unwrap();
            Self(root)
        }
    }

    impl Drop for Sandbox {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn checksum_of(contents: &[u8]) -> String {
        Sha256::digest(contents)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    #[test]
    fn replaces_the_appimage_when_the_checksum_matches() {
        let sandbox = Sandbox::new("update-ok");
        let running = sandbox.0.join("Quadra.AppImage");
        let staged = staged_path(&running);
        fs::write(&running, "old").unwrap();
        fs::write(&staged, "new").unwrap();
        let checksums = format!(
            "{}  other-file\n{}  Quadra-v1-x86_64.AppImage\n",
            checksum_of(b"x"),
            checksum_of(b"new").to_ascii_uppercase()
        );

        verify_and_install(&staged, &running, "Quadra-v1-x86_64.AppImage", &checksums).unwrap();

        assert_eq!(fs::read(&running).unwrap(), b"new");
        assert!(!staged.exists());
        assert_eq!(
            fs::metadata(&running).unwrap().permissions().mode() & 0o777,
            EXECUTABLE_MODE
        );
    }

    #[test]
    fn keeps_the_old_appimage_when_the_checksum_is_wrong_or_missing() {
        let sandbox = Sandbox::new("update-bad");
        let running = sandbox.0.join("Quadra.AppImage");
        let staged = staged_path(&running);
        fs::write(&running, "old").unwrap();
        fs::write(&staged, "tampered").unwrap();

        let wrong = format!("{}  Quadra-v1-x86_64.AppImage\n", checksum_of(b"new"));
        assert!(verify_and_install(&staged, &running, "Quadra-v1-x86_64.AppImage", &wrong).is_err());
        assert!(verify_and_install(&staged, &running, "Quadra-v1-x86_64.AppImage", "").is_err());
        assert_eq!(fs::read(&running).unwrap(), b"old");
    }

    #[test]
    fn reads_both_checksum_file_styles() {
        let text = "AAA  plain\nBBB *binary\n";
        assert_eq!(expected_checksum(text, "plain").as_deref(), Some("aaa"));
        assert_eq!(expected_checksum(text, "binary").as_deref(), Some("bbb"));
        assert_eq!(expected_checksum(text, "missing"), None);
    }

    #[test]
    fn stages_beside_the_appimage_on_the_same_filesystem() {
        assert_eq!(
            staged_path(Path::new("/home/me/Apps/Quadra.AppImage")),
            Path::new("/home/me/Apps/.Quadra.AppImage.update")
        );
    }
}
