use super::{paths, quadra_rosemary};
use crate::state::QUADRA_RUNNER;
use crate::utils::{find_in_path, path_for_display};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

pub const SYSTEM_RUNNER: &str = "System";

const BUTTER_RUNNER_MANIFEST: &str = "runner.toml";

#[derive(Debug, Clone)]
pub struct Runner {
    pub name: String,
    pub executable: PathBuf,
}

#[derive(Deserialize)]
struct ButterRunnerManifest {
    name: String,
    executable: PathBuf,
}

impl Runner {
    pub fn is_installed(&self) -> bool {
        self.executable.is_file()
    }

    pub fn is_quadra(&self) -> bool {
        self.name == QUADRA_RUNNER
    }

    pub fn with_current_executable(self) -> Self {
        if self.is_quadra() {
            Self {
                executable: quadra_rosemary::executable(),
                ..self
            }
        } else {
            self
        }
    }
}

pub fn discover(custom_runner_path: &str) -> Vec<Runner> {
    let mut runners = Vec::new();

    runners.push(Runner {
        name: QUADRA_RUNNER.to_owned(),
        executable: quadra_rosemary::executable(),
    });

    if let Some(system_executable) = find_in_path("rosemary") {
        runners.push(Runner {
            name: SYSTEM_RUNNER.to_owned(),
            executable: system_executable,
        });
    }

    for directory in paths::butter_runner_directories() {
        runners.extend(butter_runners_in(&directory));
    }

    let custom_executable = Path::new(custom_runner_path);
    if !custom_runner_path.is_empty() && custom_executable.is_file() {
        runners.push(Runner {
            name: custom_runner_name(custom_executable),
            executable: custom_executable.to_path_buf(),
        });
    }

    runners
}

pub fn find(name: &str, custom_runner_path: &str) -> Option<Runner> {
    discover(custom_runner_path)
        .into_iter()
        .find(|runner| runner.name == name)
}

pub fn custom_runner_name(executable: &Path) -> String {
    format!("Custom ({})", path_for_display(executable))
}

fn butter_runners_in(directory: &Path) -> Vec<Runner> {
    let Ok(entries) = fs::read_dir(directory) else {
        return Vec::new();
    };

    let mut runners: Vec<Runner> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|runner_directory| !is_hidden(runner_directory))
        .filter_map(|runner_directory| butter_runner_at(&runner_directory))
        .collect();
    runners.sort_by(|first, second| first.name.cmp(&second.name));
    runners
}

fn butter_runner_at(runner_directory: &Path) -> Option<Runner> {
    let manifest = fs::read_to_string(runner_directory.join(BUTTER_RUNNER_MANIFEST)).ok()?;
    let manifest: ButterRunnerManifest = toml::from_str(&manifest).ok()?;
    let executable = runner_directory.join(manifest.executable);

    executable.is_file().then(|| Runner {
        name: format!("Butter: {}", manifest.name),
        executable,
    })
}

fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .is_some_and(|name| name.to_string_lossy().starts_with('.'))
}
