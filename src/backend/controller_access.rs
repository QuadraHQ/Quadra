use super::flatpak_override;
use crate::utils::is_running_in_flatpak;

const FLATPAK_INFO: &str = "/.flatpak-info";
const CONTEXT_SECTION: &str = "[Context]";
const DEVICES_KEY: &str = "devices=";
const CONTROLLER_DEVICE: &str = "all";

pub fn is_granted() -> bool {
    let flatpak_info = std::fs::read_to_string(FLATPAK_INFO).unwrap_or_default();
    granted_devices(&flatpak_info).contains(&CONTROLLER_DEVICE)
}

pub fn command_to_change(allow_controllers: bool) -> Option<String> {
    if !is_running_in_flatpak() || allow_controllers == is_granted() {
        return None;
    }
    Some(flatpak_override::device_command(
        &flatpak_override::quadra_id(),
        CONTROLLER_DEVICE,
        allow_controllers,
    ))
}

fn granted_devices(flatpak_info: &str) -> Vec<&str> {
    flatpak_info
        .lines()
        .map(str::trim)
        .skip_while(|line| *line != CONTEXT_SECTION)
        .skip(1)
        .take_while(|line| !line.starts_with('['))
        .find_map(|line| line.strip_prefix(DEVICES_KEY))
        .map(|devices| {
            devices
                .split(';')
                .filter(|device| !device.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    const FLATPAK_INFO_WITH_ALL_DEVICES: &str = "\
[Application]
name=org.quadrahq.Quadra
runtime=runtime/org.gnome.Platform/x86_64/51

[Context]
shared=network;ipc;
sockets=x11;wayland;pulseaudio;fallback-x11;
devices=dri;all;

[Instance]
devices=nothing;
";

    #[test]
    fn reads_the_devices_granted_in_the_context() {
        assert_eq!(
            granted_devices(FLATPAK_INFO_WITH_ALL_DEVICES),
            vec!["dri", "all"]
        );
    }

    #[test]
    fn finds_no_devices_without_a_context() {
        assert!(granted_devices("[Application]\nname=org.quadrahq.Quadra\n").is_empty());
    }
}
