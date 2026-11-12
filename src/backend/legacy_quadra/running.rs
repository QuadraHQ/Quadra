use super::leftovers::LAUNCHER_NAMES;
use std::fs;
use std::path::Path;

pub fn old_quadra_is_running(processes: &Path) -> bool {
    fs::read_dir(processes)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| is_process_folder(&entry.file_name().to_string_lossy()))
        .filter_map(|entry| fs::read(entry.path().join("cmdline")).ok())
        .any(|command_line| runs_old_launcher(&command_line))
}

fn is_process_folder(name: &str) -> bool {
    !name.is_empty() && name.bytes().all(|byte| byte.is_ascii_digit())
}

fn runs_old_launcher(command_line: &[u8]) -> bool {
    command_line
        .split(|&byte| byte == 0)
        .map(String::from_utf8_lossy)
        .any(|argument| LAUNCHER_NAMES.iter().any(|name| argument.ends_with(name)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn processes_with(commands: &[(&str, &[u8])]) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!(
            "quadra-proc-{}-{}",
            std::process::id(),
            commands.len()
        ));
        let _ = fs::remove_dir_all(&root);
        for (id, command_line) in commands {
            fs::create_dir_all(root.join(id)).unwrap();
            fs::write(root.join(id).join("cmdline"), command_line).unwrap();
        }
        root
    }

    #[test]
    fn sees_the_old_launcher_among_other_processes() {
        let root = processes_with(&[
            ("100", b"/usr/bin/firefox\0--new-window\0"),
            (
                "200",
                b"python3\0/home/me/.local/share/QuadraHQ/launcher/quadrahq-launcher\0",
            ),
            ("self", b"quadrahq-launcher\0"),
        ]);
        assert!(old_quadra_is_running(&root));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn the_rewrite_and_other_programs_do_not_count() {
        let root = processes_with(&[
            ("100", b"/home/me/.local/bin/quadra-rewrite\0"),
            ("101", b"vim\0notes-about-quadrahq.txt\0"),
        ]);
        assert!(!old_quadra_is_running(&root));
        fs::remove_dir_all(root).unwrap();
    }
}
