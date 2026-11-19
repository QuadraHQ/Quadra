use super::{background::run_task, prefix};
use crate::state::AppState;
use anyhow::bail;
use std::fs;
use std::path::{Path, PathBuf};

const USERS_INSIDE_PREFIX: &str = "root/Users";
const ROBLOX_INSIDE_USER: &str = "Library/Roblox";
const TEMPORARY_STORAGE_INSIDE_PREFIX: &str = "root/private/tmp/Roblox/rbx-storage";
const CACHED_ENTRIES: [&str; 7] = [
    "OTAPatchBackups",
    "rbx-storage",
    "rbx-storage-sc",
    "rbx-storage.id",
    "rbx-storage.db",
    "rbx-storage.db-shm",
    "rbx-storage.db-wal",
];

pub fn clear(state: &AppState, when_finished: impl FnOnce(u64) + 'static) {
    if state.busy() {
        return;
    }
    if state.running() {
        state.report_error("Close Roblox before clearing its cache.");
        return;
    }
    let Some(prefix_directory) = prefix::directory_for(&state.prefix()) else {
        state.report_error("Quadra can only clear the cache of its own prefix.");
        return;
    };

    run_task(
        state,
        move |progress| {
            progress.step("Clearing the Roblox cache");
            remove_cached(&prefix_directory)
        },
        move |_, freed| when_finished(freed),
    );
}

fn remove_cached(prefix_directory: &Path) -> anyhow::Result<u64> {
    let mut freed = 0;
    let mut failures = Vec::new();
    for path in cached_paths(prefix_directory) {
        let size = size_of(&path);
        match remove(&path) {
            Ok(()) => freed += size,
            Err(error) => failures.push(format!("{}: {error}", path.display())),
        }
    }
    if !failures.is_empty() {
        bail!("Couldn't clear the cache:\n{}", failures.join("\n"));
    }
    Ok(freed)
}

fn cached_paths(prefix_directory: &Path) -> Vec<PathBuf> {
    let users = prefix_directory.join(USERS_INSIDE_PREFIX);
    let mut paths = vec![prefix_directory.join(TEMPORARY_STORAGE_INSIDE_PREFIX)];
    for user in fs::read_dir(users).into_iter().flatten().flatten() {
        let roblox = user.path().join(ROBLOX_INSIDE_USER);
        paths.extend(CACHED_ENTRIES.iter().map(|entry| roblox.join(entry)));
    }
    paths.retain(|path| path.symlink_metadata().is_ok());
    paths
}

fn remove(path: &Path) -> std::io::Result<()> {
    if path.symlink_metadata()?.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

fn size_of(path: &Path) -> u64 {
    let Ok(metadata) = path.symlink_metadata() else {
        return 0;
    };
    if !metadata.is_dir() {
        return metadata.len();
    }
    fs::read_dir(path)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| size_of(&entry.path()))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clears_only_the_cache_and_counts_what_it_freed() {
        let root = std::env::temp_dir().join(format!("quadra-cache-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let roblox = root
            .join(USERS_INSIDE_PREFIX)
            .join("someone")
            .join(ROBLOX_INSIDE_USER);
        fs::create_dir_all(roblox.join("OTAPatchBackups/ui")).unwrap();
        fs::create_dir_all(roblox.join("LocalStorage")).unwrap();
        fs::write(roblox.join("OTAPatchBackups/ui/patch"), [0; 100]).unwrap();
        fs::write(roblox.join("rbx-storage.db"), [0; 50]).unwrap();
        fs::write(roblox.join("LocalStorage/keep"), "kept").unwrap();
        let temporary = root.join(TEMPORARY_STORAGE_INSIDE_PREFIX);
        fs::create_dir_all(&temporary).unwrap();
        fs::write(temporary.join("asset"), [0; 25]).unwrap();

        assert_eq!(remove_cached(&root).unwrap(), 175);
        assert!(!roblox.join("OTAPatchBackups").exists());
        assert!(!roblox.join("rbx-storage.db").exists());
        assert!(!temporary.exists());
        assert!(roblox.join("LocalStorage/keep").exists());
        assert_eq!(remove_cached(&root).unwrap(), 0);
        let _ = fs::remove_dir_all(&root);
    }
}
