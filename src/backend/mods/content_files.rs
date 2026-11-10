use crate::utils::{create_parent_directory, files_under};
use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const ORIGINALS_DIRECTORY: &str = "originals";
const CREATED_FILE: &str = "created.json";

pub struct ContentFiles {
    content: PathBuf,
    backups: PathBuf,
    created: BTreeSet<PathBuf>,
}

impl ContentFiles {
    pub fn open_clean(content: PathBuf, backups: PathBuf) -> io::Result<Self> {
        restore_originals(&content, &backups)?;
        Ok(Self {
            content,
            backups,
            created: BTreeSet::new(),
        })
    }

    pub fn exists(&self, relative: &Path) -> bool {
        self.content.join(relative).is_file()
    }

    pub fn read(&self, relative: &Path) -> io::Result<Vec<u8>> {
        fs::read(self.content.join(relative))
    }

    pub fn list(&self, relative_directory: &Path) -> Vec<PathBuf> {
        fs::read_dir(self.content.join(relative_directory))
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| relative_directory.join(entry.file_name()))
            .collect()
    }

    pub fn place_bytes(&mut self, relative: &Path, bytes: &[u8]) -> io::Result<()> {
        let target = self.prepare(relative)?;
        fs::write(target, bytes)
    }

    pub fn place_file(&mut self, relative: &Path, source: &Path) -> io::Result<()> {
        let target = self.prepare(relative)?;
        fs::copy(source, target).map(drop)
    }

    pub fn remove(&mut self, relative: &Path) -> io::Result<()> {
        if !self.exists(relative) {
            return Ok(());
        }
        self.keep_original(relative)?;
        fs::remove_file(self.content.join(relative))
    }

    pub fn finish(self) -> io::Result<()> {
        if self.created.is_empty() && !self.backups.exists() {
            return Ok(());
        }
        fs::create_dir_all(&self.backups)?;
        let created = serde_json::to_vec(&self.created).map_err(io::Error::other)?;
        fs::write(self.backups.join(CREATED_FILE), created)
    }

    fn prepare(&mut self, relative: &Path) -> io::Result<PathBuf> {
        self.keep_original(relative)?;
        let target = self.content.join(relative);
        create_parent_directory(&target)?;
        Ok(target)
    }

    fn keep_original(&mut self, relative: &Path) -> io::Result<()> {
        if self.created.contains(relative) {
            return Ok(());
        }
        let original = self.content.join(relative);
        if !original.is_file() {
            self.created.insert(relative.to_owned());
            return Ok(());
        }
        let kept = self.backups.join(ORIGINALS_DIRECTORY).join(relative);
        if kept.exists() {
            return Ok(());
        }
        create_parent_directory(&kept)?;
        fs::copy(original, kept).map(drop)
    }
}

fn restore_originals(content: &Path, backups: &Path) -> io::Result<()> {
    if !backups.is_dir() {
        return Ok(());
    }

    let created: BTreeSet<PathBuf> = fs::read(backups.join(CREATED_FILE))
        .ok()
        .and_then(|contents| serde_json::from_slice(&contents).ok())
        .unwrap_or_default();
    for relative in created {
        let _ = fs::remove_file(content.join(relative));
    }

    let originals = backups.join(ORIGINALS_DIRECTORY);
    if originals.is_dir() {
        for kept in files_under(&originals)? {
            if let Ok(relative) = kept.strip_prefix(&originals) {
                let original = content.join(relative);
                create_parent_directory(&original)?;
                fs::copy(&kept, original)?;
            }
        }
    }

    fs::remove_dir_all(backups)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Sandbox(PathBuf);

    impl Sandbox {
        fn new(name: &str) -> Self {
            let root = std::env::temp_dir().join(format!("quadra-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(root.join("content/sounds")).unwrap();
            Self(root)
        }

        fn content(&self) -> PathBuf {
            self.0.join("content")
        }

        fn open(&self) -> ContentFiles {
            ContentFiles::open_clean(self.content(), self.0.join("backups")).unwrap()
        }
    }

    impl Drop for Sandbox {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn restores_replaced_and_deleted_originals_and_removes_created_files() {
        let sandbox = Sandbox::new("restore");
        fs::write(sandbox.content().join("sounds/ouch.ogg"), "original").unwrap();
        fs::write(sandbox.content().join("sounds/oof.ogg"), "kept").unwrap();

        let mut files = sandbox.open();
        files
            .place_bytes(Path::new("sounds/ouch.ogg"), b"first")
            .unwrap();
        files
            .place_bytes(Path::new("sounds/ouch.ogg"), b"second")
            .unwrap();
        files
            .place_bytes(Path::new("sounds/new.ogg"), b"new")
            .unwrap();
        files
            .place_bytes(Path::new("sounds/new.ogg"), b"newer")
            .unwrap();
        files.remove(Path::new("sounds/oof.ogg")).unwrap();
        files.finish().unwrap();

        assert_eq!(
            fs::read(sandbox.content().join("sounds/ouch.ogg")).unwrap(),
            b"second"
        );
        assert!(!sandbox.content().join("sounds/oof.ogg").exists());

        sandbox.open().finish().unwrap();

        assert_eq!(
            fs::read(sandbox.content().join("sounds/ouch.ogg")).unwrap(),
            b"original"
        );
        assert_eq!(
            fs::read(sandbox.content().join("sounds/oof.ogg")).unwrap(),
            b"kept"
        );
        assert!(!sandbox.content().join("sounds/new.ogg").exists());
    }

    #[test]
    fn applying_twice_does_not_back_up_a_modified_file() {
        let sandbox = Sandbox::new("twice");
        fs::write(sandbox.content().join("sounds/ouch.ogg"), "original").unwrap();

        for _ in 0..2 {
            let mut files = sandbox.open();
            files
                .place_bytes(Path::new("sounds/ouch.ogg"), b"mod")
                .unwrap();
            files.finish().unwrap();
        }
        sandbox.open().finish().unwrap();

        assert_eq!(
            fs::read(sandbox.content().join("sounds/ouch.ogg")).unwrap(),
            b"original"
        );
    }
}
