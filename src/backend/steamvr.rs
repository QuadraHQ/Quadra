use super::paths;
use gtk::glib;
use std::fs;
use std::path::{Path, PathBuf};

const STEAM_ROOTS_IN_HOME: [&str; 4] = [
    ".local/share/Steam",
    ".steam/steam",
    ".steam/root",
    ".steam/debian-installation",
];
const LIBRARY_LIST: &str = "steamapps/libraryfolders.vdf";
const LIBRARY_PATH_KEY: &str = "\"path\"";
const RUNTIME_MANIFEST_IN_LIBRARY: &str = "steamapps/common/SteamVR/steamxr_linux64.json";
const SERVER_PROCESS: &str = "vrserver";
const PROCESSES: &str = "/proc";

pub fn runtime_manifests() -> Vec<PathBuf> {
    let home = glib::home_dir();
    manifests_under(STEAM_ROOTS_IN_HOME.map(|root| home.join(root)))
}

pub fn is_running() -> bool {
    has_process_named(Path::new(PROCESSES), SERVER_PROCESS)
}

fn manifests_under(steam_roots: impl IntoIterator<Item = PathBuf>) -> Vec<PathBuf> {
    let mut manifests = Vec::new();
    for root in steam_roots {
        let listed = paths::resolved_on_host_os(&root.join(LIBRARY_LIST))
            .and_then(|list| fs::read_to_string(list).ok())
            .map(|list| library_paths(&list))
            .unwrap_or_default();
        for library in std::iter::once(root).chain(listed) {
            let Some(manifest) =
                paths::resolved_on_host_os(&library.join(RUNTIME_MANIFEST_IN_LIBRARY))
            else {
                continue;
            };
            if !manifests.contains(&manifest) {
                manifests.push(manifest);
            }
        }
    }
    manifests
}

fn library_paths(library_list: &str) -> Vec<PathBuf> {
    library_list
        .lines()
        .filter_map(|line| line.trim().strip_prefix(LIBRARY_PATH_KEY))
        .filter_map(|value| value.trim().strip_prefix('"')?.strip_suffix('"'))
        .map(|path| PathBuf::from(path.replace("\\\\", "\\").replace("\\\"", "\"")))
        .collect()
}

fn has_process_named(processes: &Path, name: &str) -> bool {
    let Ok(entries) = fs::read_dir(processes) else {
        return false;
    };
    entries.flatten().any(|process| {
        fs::read_to_string(process.path().join("comm"))
            .is_ok_and(|command| command.trim_end() == name)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let directory =
                std::env::temp_dir().join(format!("quadra-steamvr-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&directory);
            fs::create_dir_all(&directory).unwrap();
            Self(directory.canonicalize().unwrap())
        }

        fn file(&self, path: &str, contents: &str) -> PathBuf {
            let path = self.0.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, contents).unwrap();
            path
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn finds_steamvr_in_a_library_on_another_drive() {
        let scratch = Scratch::new("library");
        let library = scratch.0.join("drive/SteamLibrary");
        scratch.file(
            "Steam/steamapps/libraryfolders.vdf",
            &format!(
                "\"libraryfolders\"\n{{\n\t\"1\"\n\t{{\n\t\t\"path\"\t\t\"{}\"\n\t}}\n}}\n",
                library.display()
            ),
        );
        let manifest = scratch.file(
            "drive/SteamLibrary/steamapps/common/SteamVR/steamxr_linux64.json",
            "{}",
        );

        let found = manifests_under([scratch.0.join("Steam"), scratch.0.join("Steam")]);

        assert_eq!(found, [manifest]);
    }

    #[test]
    fn finds_nothing_without_steamvr() {
        let scratch = Scratch::new("absent");

        assert!(manifests_under([scratch.0.join("Steam")]).is_empty());
    }

    #[test]
    fn notices_a_process_by_its_name() {
        let scratch = Scratch::new("processes");
        scratch.file("12/comm", "vrserver\n");

        assert!(has_process_named(&scratch.0, "vrserver"));
        assert!(!has_process_named(&scratch.0, "vrmonitor"));
    }
}
