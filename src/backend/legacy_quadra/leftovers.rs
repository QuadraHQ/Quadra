use gtk::glib;
use std::fs;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

pub const LAUNCHER_NAMES: [&str; 3] =
    ["quadrahq-launcher", "ciderhq-launcher", "macoblox-launcher"];

const INSTALL_FOLDERS: [(&str, &str); 3] = [
    ("QuadraHQ", "quadrahq-launcher"),
    ("CiderHQ", "ciderhq-launcher"),
    ("MacOBlox", "macoblox-launcher"),
];
const LAUNCHER_FOLDER: &str = "launcher";
const OLD_FOLDER: &str = "quadrahq";
const OLD_RUNTIME_FOLDERS: [&str; 2] = ["rosemary", "rosemary-runner"];
const COMMAND_LINK: &str = "quadrahq";
const MAXIMUM_SHORTCUT_BYTES: u64 = 64 * 1024;
const BYTES_PER_BLOCK: u64 = 512;

pub struct Roots {
    pub data: PathBuf,
    pub cache: PathBuf,
    pub config: PathBuf,
    pub bin: PathBuf,
    pub applications: PathBuf,
}

impl Roots {
    pub fn of_user() -> Self {
        Self {
            data: glib::user_data_dir(),
            cache: glib::user_cache_dir(),
            config: glib::user_config_dir(),
            bin: glib::home_dir().join(".local").join("bin"),
            applications: glib::user_data_dir().join("applications"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Leftover {
    pub path: PathBuf,
    pub bytes: u64,
}

pub fn find(roots: &Roots) -> Vec<Leftover> {
    let mut paths = installs(roots);
    paths.extend(runtimes(roots));
    paths.extend([roots.cache.join(OLD_FOLDER), roots.config.join(OLD_FOLDER)]);
    paths.retain(|path| is_real_directory(path));
    paths.extend(command_link(roots));
    paths.extend(shortcuts(roots));

    paths
        .into_iter()
        .map(|path| Leftover {
            bytes: size_of(&path),
            path,
        })
        .collect()
}

pub fn remove(leftovers: &[Leftover]) -> Vec<String> {
    leftovers
        .iter()
        .filter_map(|leftover| {
            remove_one(&leftover.path)
                .err()
                .map(|error| format!("{}: {error}", leftover.path.display()))
        })
        .collect()
}

fn remove_one(path: &Path) -> io::Result<()> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    if metadata.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

fn installs(roots: &Roots) -> Vec<PathBuf> {
    INSTALL_FOLDERS
        .iter()
        .map(|(folder, launcher)| (roots.data.join(folder), launcher))
        .filter(|(folder, launcher)| folder.join(LAUNCHER_FOLDER).join(launcher).is_file())
        .map(|(folder, _)| folder)
        .collect()
}

fn runtimes(roots: &Roots) -> Vec<PathBuf> {
    OLD_RUNTIME_FOLDERS
        .iter()
        .map(|folder| roots.data.join(OLD_FOLDER).join(folder))
        .collect()
}

fn command_link(roots: &Roots) -> Option<PathBuf> {
    let link = roots.bin.join(COMMAND_LINK);
    let target = fs::read_link(&link).ok()?;
    names_old_launcher(&target.to_string_lossy()).then_some(link)
}

fn shortcuts(roots: &Roots) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = fs::read_dir(&roots.applications)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "desktop")
        })
        .filter(|path| runs_old_launcher(path))
        .collect();
    found.sort();
    found
}

fn runs_old_launcher(shortcut: &Path) -> bool {
    let Ok(metadata) = fs::symlink_metadata(shortcut) else {
        return false;
    };
    if !metadata.is_file() || metadata.len() > MAXIMUM_SHORTCUT_BYTES {
        return false;
    }
    fs::read_to_string(shortcut).is_ok_and(|text| {
        text.lines()
            .any(|line| line.starts_with("Exec=") && names_old_launcher(line))
    })
}

fn names_old_launcher(text: &str) -> bool {
    LAUNCHER_NAMES.iter().any(|name| text.contains(name))
}

fn is_real_directory(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|metadata| metadata.is_dir())
}

fn size_of(path: &Path) -> u64 {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return 0;
    };
    let own = metadata.blocks() * BYTES_PER_BLOCK;
    if !metadata.is_dir() {
        return own;
    }
    own + fs::read_dir(path)
        .into_iter()
        .flatten()
        .flatten()
        .map(|child| size_of(&child.path()))
        .sum::<u64>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    struct Sandbox(PathBuf);

    impl Sandbox {
        fn new(name: &str) -> Self {
            let root = std::env::temp_dir().join(format!("quadra-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            for folder in ["data", "cache", "config", "bin", "data/applications"] {
                fs::create_dir_all(root.join(folder)).unwrap();
            }
            Self(root)
        }

        fn roots(&self) -> Roots {
            Roots {
                data: self.0.join("data"),
                cache: self.0.join("cache"),
                config: self.0.join("config"),
                bin: self.0.join("bin"),
                applications: self.0.join("data/applications"),
            }
        }

        fn write(&self, relative: &str, contents: &str) {
            let path = self.0.join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, contents).unwrap();
        }
    }

    impl Drop for Sandbox {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn names(sandbox: &Sandbox, leftovers: &[Leftover]) -> Vec<String> {
        let mut names: Vec<String> = leftovers
            .iter()
            .map(|leftover| {
                leftover
                    .path
                    .strip_prefix(&sandbox.0)
                    .unwrap()
                    .display()
                    .to_string()
            })
            .collect();
        names.sort();
        names
    }

    fn old_quadra(sandbox: &Sandbox) {
        sandbox.write(
            "data/QuadraHQ/launcher/quadrahq-launcher",
            "#!/usr/bin/python3",
        );
        sandbox.write("data/QuadraHQ/RobloxPlayer.app/big", &"x".repeat(100_000));
        sandbox.write("data/quadrahq/rosemary/bin/rosemary", "r");
        sandbox.write("data/quadrahq/rosemary-runner/prefix/file", "p");
        sandbox.write("cache/quadrahq/metal-target/file", "c");
        sandbox.write("config/quadrahq/settings.json", "{}");
        symlink(
            sandbox.0.join("data/QuadraHQ/launcher/quadrahq-launcher"),
            sandbox.0.join("bin/quadrahq"),
        )
        .unwrap();
        sandbox.write(
            "data/applications/org.quadrahq.Quadra.URI.desktop",
            "[Desktop Entry]\nExec=\"/x/QuadraHQ/launcher/quadrahq-launcher\" %u\n",
        );
    }

    #[test]
    fn finds_everything_the_old_quadra_left_and_nothing_else() {
        let sandbox = Sandbox::new("legacy-find");
        old_quadra(&sandbox);
        sandbox.write(
            "data/quadrahq/rust-rebuild/quadra-rewrite/Cargo.toml",
            "keep",
        );
        sandbox.write("data/quadrahq/vr-emulator/run-service.sh", "keep");
        sandbox.write("data/quadra/prefix/file", "the rewrite's own data");
        sandbox.write("config/quadra/config.toml", "the rewrite's own settings");
        sandbox.write(
            "data/applications/org.quadrahq.Quadra.desktop",
            "[Desktop Entry]\nExec=/home/me/.local/bin/quadra-rewrite\n",
        );
        sandbox.write(
            "data/applications/firefox.desktop",
            "[Desktop Entry]\nExec=firefox\n",
        );

        assert_eq!(
            names(&sandbox, &find(&sandbox.roots())),
            [
                "bin/quadrahq",
                "cache/quadrahq",
                "config/quadrahq",
                "data/QuadraHQ",
                "data/applications/org.quadrahq.Quadra.URI.desktop",
                "data/quadrahq/rosemary",
                "data/quadrahq/rosemary-runner",
            ]
        );
    }

    #[test]
    fn a_clean_machine_has_nothing_to_offer() {
        let sandbox = Sandbox::new("legacy-clean");
        sandbox.write("data/quadra/prefix/file", "x");
        assert!(find(&sandbox.roots()).is_empty());
    }

    #[test]
    fn a_folder_with_the_old_name_but_no_old_launcher_is_left_alone() {
        let sandbox = Sandbox::new("legacy-unrelated");
        sandbox.write("data/QuadraHQ/notes.txt", "someone's own folder");
        assert!(find(&sandbox.roots()).is_empty());
    }

    #[test]
    fn measures_what_it_would_free() {
        let sandbox = Sandbox::new("legacy-size");
        old_quadra(&sandbox);
        let found = find(&sandbox.roots());
        let install = found
            .iter()
            .find(|leftover| leftover.path.ends_with("data/QuadraHQ"))
            .unwrap();
        assert!(install.bytes >= 100_000, "{}", install.bytes);
    }

    #[test]
    fn removes_what_was_found_and_keeps_the_rest() {
        let sandbox = Sandbox::new("legacy-remove");
        old_quadra(&sandbox);
        sandbox.write("data/quadrahq/rust-rebuild/Cargo.toml", "keep");
        sandbox.write("data/quadra/prefix/file", "keep");

        let found = find(&sandbox.roots());
        assert!(remove(&found).is_empty());

        assert!(!sandbox.0.join("data/QuadraHQ").exists());
        assert!(!sandbox.0.join("data/quadrahq/rosemary").exists());
        assert!(!sandbox.0.join("cache/quadrahq").exists());
        assert!(fs::symlink_metadata(sandbox.0.join("bin/quadrahq")).is_err());
        assert!(
            sandbox
                .0
                .join("data/quadrahq/rust-rebuild/Cargo.toml")
                .exists()
        );
        assert!(sandbox.0.join("data/quadra/prefix/file").exists());
        assert!(find(&sandbox.roots()).is_empty());
    }

    #[test]
    fn never_follows_a_symlink_out_of_the_old_folders() {
        let sandbox = Sandbox::new("legacy-symlink");
        old_quadra(&sandbox);
        sandbox.write("elsewhere/precious.txt", "do not delete");
        symlink(
            sandbox.0.join("elsewhere"),
            sandbox.0.join("data/QuadraHQ/linked"),
        )
        .unwrap();
        fs::remove_dir_all(sandbox.0.join("cache/quadrahq")).unwrap();
        symlink(
            sandbox.0.join("elsewhere"),
            sandbox.0.join("cache/quadrahq"),
        )
        .unwrap();

        let found = find(&sandbox.roots());
        assert!(
            !found
                .iter()
                .any(|leftover| leftover.path.ends_with("cache/quadrahq"))
        );
        remove(&found);

        assert!(sandbox.0.join("elsewhere/precious.txt").exists());
        assert!(!sandbox.0.join("data/QuadraHQ").exists());
    }

    #[test]
    fn removing_something_already_gone_is_not_an_error() {
        let sandbox = Sandbox::new("legacy-gone");
        let gone = Leftover {
            path: sandbox.0.join("data/QuadraHQ"),
            bytes: 1,
        };
        assert!(remove(&[gone]).is_empty());
    }
}
