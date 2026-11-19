use super::flatpak_override;
use crate::utils::is_running_in_flatpak;
use std::fs;

const SANDBOX_INFO: &str = "/.flatpak-info";
const GRANTED_FILESYSTEMS_KEY: &str = "filesystems=";

const PERMISSIONS: [&str; 6] = [
    "host-os:ro",
    "xdg-config/openxr:ro",
    "xdg-run/monado_comp_ipc",
    "xdg-run/wivrn:create",
    "xdg-data/flatpak/app/io.github.wivrn.wivrn:ro",
    "/var/lib/flatpak/app/io.github.wivrn.wivrn:ro",
];

pub fn missing() -> Option<String> {
    if !is_running_in_flatpak() {
        return None;
    }
    let sandbox_info = fs::read_to_string(SANDBOX_INFO).unwrap_or_default();
    missing_from(&sandbox_info, &flatpak_override::quadra_id())
}

fn missing_from(sandbox_info: &str, flatpak_id: &str) -> Option<String> {
    let granted: Vec<&str> = sandbox_info
        .lines()
        .find_map(|line| line.strip_prefix(GRANTED_FILESYSTEMS_KEY))
        .unwrap_or_default()
        .split(';')
        .map(without_access_mode)
        .collect();
    let missing: Vec<&str> = PERMISSIONS
        .into_iter()
        .filter(|permission| !granted.contains(&without_access_mode(permission)))
        .collect();

    if missing.is_empty() {
        None
    } else {
        Some(flatpak_override::command(flatpak_id, missing))
    }
}

fn without_access_mode(permission: &str) -> &str {
    permission
        .rsplit_once(':')
        .map_or(permission, |(filesystem, _)| filesystem)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FLATPAK_ID: &str = "org.quadrahq.Quadra";

    fn sandbox_info_granting(filesystems: &str) -> String {
        format!(
            "[Application]\nname={FLATPAK_ID}\n\n[Context]\nshared=network;\n\
             devices=dri;\nfilesystems={filesystems}\n\n[Instance]\ninstance-id=1\n"
        )
    }

    #[test]
    fn asks_for_everything_in_an_untouched_sandbox() {
        assert_eq!(
            missing_from("[Context]\nshared=network;\n", FLATPAK_ID),
            Some(
                "flatpak override --user \
                 --filesystem=host-os:ro \
                 --filesystem=xdg-config/openxr:ro \
                 --filesystem=xdg-run/monado_comp_ipc \
                 --filesystem=xdg-run/wivrn:create \
                 --filesystem=xdg-data/flatpak/app/io.github.wivrn.wivrn:ro \
                 --filesystem=/var/lib/flatpak/app/io.github.wivrn.wivrn:ro \
                 org.quadrahq.Quadra"
                    .to_owned()
            )
        );
    }

    #[test]
    fn asks_only_for_what_is_not_granted_yet() {
        let sandbox_info = sandbox_info_granting(
            "home;host-os:ro;xdg-config/openxr:ro;xdg-run/monado_comp_ipc;xdg-run/wivrn;",
        );
        assert_eq!(
            missing_from(&sandbox_info, FLATPAK_ID),
            Some(
                "flatpak override --user \
                 --filesystem=xdg-data/flatpak/app/io.github.wivrn.wivrn:ro \
                 --filesystem=/var/lib/flatpak/app/io.github.wivrn.wivrn:ro \
                 org.quadrahq.Quadra"
                    .to_owned()
            )
        );
    }

    #[test]
    fn asks_for_nothing_once_everything_is_granted() {
        let sandbox_info = sandbox_info_granting(
            "host-os:ro;xdg-config/openxr:ro;xdg-run/monado_comp_ipc;xdg-run/wivrn:create;\
             xdg-data/flatpak/app/io.github.wivrn.wivrn:ro;\
             /var/lib/flatpak/app/io.github.wivrn.wivrn:ro;",
        );
        assert_eq!(missing_from(&sandbox_info, FLATPAK_ID), None);
    }
}
