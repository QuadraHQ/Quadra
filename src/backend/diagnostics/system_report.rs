use crate::backend::{self, paths, roblox};
use crate::state::{AppState, Client};
use std::fmt::Write;
use std::fs;
use std::path::Path;

const ENVIRONMENT_PREFIXES: [&str; 11] = [
    "ROSEMARY_",
    "MESA_",
    "VK_",
    "MANGOHUD",
    "__GL",
    "__NV",
    "LIBGL_",
    "ZINK_",
    "XDG_SESSION_TYPE",
    "XDG_CURRENT_DESKTOP",
    "WAYLAND_DISPLAY",
];
const VENDORS: [(&str, &str); 3] = [("0x10de", "NVIDIA"), ("0x1002", "AMD"), ("0x8086", "Intel")];

pub fn quadra_section(state: &AppState) -> String {
    let mut report = String::new();
    let _ = writeln!(report, "Quadra {}", env!("CARGO_PKG_VERSION"));
    let _ = writeln!(report, "Rosemary: {}", backend::QUADRA_ROSEMARY_VERSION);
    let _ = writeln!(report, "Runner: {}", state.runner());
    let _ = writeln!(report, "Prefix: {}", state.prefix());
    let _ = writeln!(report, "Renderer: {:?}", state.renderer());
    let _ = writeln!(report, "Flatpak: {}", crate::utils::is_running_in_flatpak());
    for client in Client::ALL {
        let version = roblox::installed_release(client).map(|release| release.version);
        let _ = writeln!(
            report,
            "{}: {}",
            client.name(),
            version.as_deref().unwrap_or("not installed")
        );
    }
    report
}

pub fn hardware_section() -> String {
    let mut report = String::new();
    let _ = writeln!(report, "OS: {}", os_name());
    let _ = writeln!(report, "Kernel: {}", first_line(Path::new("/proc/version")));
    let _ = writeln!(report, "CPU: {}", cpu_description());
    let _ = writeln!(report, "Memory: {}", memory_description());
    for gpu in gpus() {
        let _ = writeln!(report, "GPU: {gpu}");
    }
    let _ = writeln!(report, "Vulkan drivers: {}", vulkan_drivers().join(", "));
    report
}

pub fn environment_section() -> String {
    let mut variables: Vec<(String, String)> = std::env::vars()
        .filter(|(name, _)| {
            ENVIRONMENT_PREFIXES
                .iter()
                .any(|prefix| name.starts_with(prefix))
        })
        .collect();
    variables.sort();
    variables
        .into_iter()
        .map(|(name, value)| format!("{name}={value}\n"))
        .collect()
}

pub fn log_file() -> std::path::PathBuf {
    paths::logs_directory().join("roblox.log")
}

fn os_name() -> String {
    fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|release| {
            release
                .lines()
                .find_map(|line| line.strip_prefix("PRETTY_NAME="))
                .map(|name| name.trim_matches('"').to_owned())
        })
        .unwrap_or_else(|| "unknown".to_owned())
}

fn first_line(path: &Path) -> String {
    fs::read_to_string(path)
        .ok()
        .and_then(|contents| contents.lines().next().map(str::to_owned))
        .unwrap_or_else(|| "unknown".to_owned())
}

fn cpu_description() -> String {
    let cpuinfo = fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    let model = cpuinfo
        .lines()
        .find_map(|line| line.strip_prefix("model name"))
        .and_then(|line| line.split_once(':'))
        .map_or("unknown", |(_, model)| model.trim());
    let threads = std::thread::available_parallelism().map_or(0, |threads| threads.get());
    format!("{model} ({threads} threads)")
}

fn memory_description() -> String {
    let meminfo = fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let total_kib: u64 = meminfo
        .lines()
        .find_map(|line| line.strip_prefix("MemTotal:"))
        .and_then(|line| line.split_whitespace().next())
        .and_then(|kib| kib.parse().ok())
        .unwrap_or_default();
    format!("{} MiB", total_kib / 1024)
}

fn gpus() -> Vec<String> {
    let mut gpus: Vec<String> = fs::read_dir("/sys/class/drm")
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| is_card(&entry.file_name().to_string_lossy()))
        .map(|entry| describe_gpu(&entry.path().join("device")))
        .collect();
    gpus.sort();
    gpus
}

fn is_card(name: &str) -> bool {
    name.strip_prefix("card")
        .is_some_and(|number| !number.is_empty() && number.chars().all(|c| c.is_ascii_digit()))
}

fn describe_gpu(device: &Path) -> String {
    let attribute = |name: &str| first_line(&device.join(name));
    let vendor_id = attribute("vendor");
    let vendor = VENDORS
        .iter()
        .find(|(id, _)| *id == vendor_id)
        .map_or(vendor_id.as_str(), |(_, name)| name);
    let driver = fs::read_link(device.join("driver")).ok().and_then(|link| {
        link.file_name()
            .map(|name| name.to_string_lossy().into_owned())
    });
    format!(
        "{vendor} {} (driver {})",
        attribute("device"),
        driver.as_deref().unwrap_or("unknown")
    )
}

fn vulkan_drivers() -> Vec<String> {
    let mut drivers: Vec<String> = ["/usr/share/vulkan/icd.d", "/etc/vulkan/icd.d"]
        .iter()
        .flat_map(|directory| fs::read_dir(directory).into_iter().flatten().flatten())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    drivers.sort();
    drivers
}

#[cfg(test)]
mod tests {
    use super::is_card;

    #[test]
    fn recognises_only_gpu_card_entries() {
        assert!(is_card("card0"));
        assert!(!is_card("card0-HDMI-A-1"));
        assert!(!is_card("renderD128"));
        assert!(!is_card("card"));
    }
}
