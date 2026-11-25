use super::paths;
use crate::utils::{find_in_path, is_executable_file};
use gtk::glib;
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Child, Command};

const PROGRAM: &str = "quadra-web-window";
const BRIDGE_VARIABLE: &str = "ROSEMARY_WEBKIT_BRIDGE";
const DEFAULT_PREFIX_DATA: &str = "default";
const PREFIX_DIGEST_BYTES: usize = 8;

pub struct WebWindow {
    program: PathBuf,
    socket: PathBuf,
    data_directory: PathBuf,
}

pub struct RunningWebWindow {
    process: Child,
    socket: PathBuf,
}

impl WebWindow {
    pub fn for_prefix(prefix_directory: Option<&Path>) -> Option<Self> {
        Some(Self {
            program: program()?,
            socket: glib::user_runtime_dir().join(format!("quadra-web-{}.sock", process::id())),
            data_directory: paths::web_window_directory().join(data_name(prefix_directory)),
        })
    }

    pub fn rosemary_environment(&self) -> (&'static str, &Path) {
        (BRIDGE_VARIABLE, &self.socket)
    }

    pub fn command(&self) -> Command {
        let mut command = Command::new(&self.program);
        command.arg(&self.socket).arg(&self.data_directory);
        command
    }

    pub fn running(self, process: Child) -> RunningWebWindow {
        RunningWebWindow {
            process,
            socket: self.socket,
        }
    }
}

impl Drop for RunningWebWindow {
    fn drop(&mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();
        let _ = fs::remove_file(&self.socket);
    }
}

pub fn is_installed() -> bool {
    program().is_some()
}

fn program() -> Option<PathBuf> {
    let beside_quadra = env::current_exe()
        .ok()
        .map(|quadra| quadra.with_file_name(PROGRAM))
        .filter(|program| is_executable_file(program));
    beside_quadra.or_else(|| find_in_path(PROGRAM))
}

fn data_name(prefix_directory: Option<&Path>) -> String {
    let Some(prefix_directory) = prefix_directory else {
        return DEFAULT_PREFIX_DATA.to_owned();
    };
    Sha256::digest(prefix_directory.as_os_str().as_encoded_bytes())
        .iter()
        .take(PREFIX_DIGEST_BYTES)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_prefix_keeps_its_own_web_sign_in() {
        let first = data_name(Some(Path::new("/prefixes/first")));

        assert_eq!(first, data_name(Some(Path::new("/prefixes/first"))));
        assert_ne!(first, data_name(Some(Path::new("/prefixes/second"))));
        assert_ne!(first, data_name(None));
        assert_eq!(first.len(), PREFIX_DIGEST_BYTES * 2);
    }
}
