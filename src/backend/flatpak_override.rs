const QUADRA_FLATPAK_ID: &str = "org.quadrahq.Quadra";

pub fn quadra_id() -> String {
    std::env::var("FLATPAK_ID").unwrap_or_else(|_| QUADRA_FLATPAK_ID.to_owned())
}

pub fn command<'a>(flatpak_id: &str, permissions: impl IntoIterator<Item = &'a str>) -> String {
    let mut granted: Vec<&str> = Vec::new();
    for permission in permissions {
        if !granted.contains(&permission) {
            granted.push(permission);
        }
    }

    let mut command = String::from("flatpak override --user");
    for permission in granted {
        command.push_str(" --filesystem=");
        command.push_str(permission);
    }
    command.push(' ');
    command.push_str(flatpak_id);
    command
}

pub fn device_command(flatpak_id: &str, device: &str, is_allowed: bool) -> String {
    let flag = if is_allowed { "--device" } else { "--nodevice" };
    format!("flatpak override --user {flag}={device} {flatpak_id}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grants_each_path_once() {
        assert_eq!(
            command(
                QUADRA_FLATPAK_ID,
                ["xdg-run/a", "xdg-run/b:ro", "xdg-run/a"]
            ),
            "flatpak override --user --filesystem=xdg-run/a --filesystem=xdg-run/b:ro \
             org.quadrahq.Quadra"
        );
    }

    #[test]
    fn allows_and_disallows_a_device() {
        assert_eq!(
            device_command(QUADRA_FLATPAK_ID, "all", true),
            "flatpak override --user --device=all org.quadrahq.Quadra"
        );
        assert_eq!(
            device_command(QUADRA_FLATPAK_ID, "all", false),
            "flatpak override --user --nodevice=all org.quadrahq.Quadra"
        );
    }
}
