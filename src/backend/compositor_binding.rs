use gtk::gdk;

const MODIFIER_NAMES: [(gdk::ModifierType, &str); 4] = [
    (gdk::ModifierType::CONTROL_MASK, "Ctrl"),
    (gdk::ModifierType::ALT_MASK, "Alt"),
    (gdk::ModifierType::SUPER_MASK, "Super"),
    (gdk::ModifierType::SHIFT_MASK, "Shift"),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompositorBinding {
    pub compositor: &'static str,
    pub line: String,
}

struct Combination {
    modifiers: Vec<&'static str>,
    key: String,
}

pub fn for_current_desktop(accelerator: &str, command: &[String]) -> Option<CompositorBinding> {
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    let combination = combination_of(accelerator)?;
    binding_for(&desktop, &combination, command)
}

fn combination_of(accelerator: &str) -> Option<Combination> {
    let (key, modifiers) = gtk::accelerator_parse(accelerator)?;
    let key = key.name()?;
    Some(Combination {
        modifiers: MODIFIER_NAMES
            .iter()
            .filter(|(modifier, _)| modifiers.contains(*modifier))
            .map(|(_, name)| *name)
            .collect(),
        key: key.to_string(),
    })
}

fn binding_for(
    desktop: &str,
    combination: &Combination,
    command: &[String],
) -> Option<CompositorBinding> {
    let desktop = desktop.to_lowercase();
    let toggle_overlay = "--toggle-overlay";
    let key = combination.key.to_uppercase();
    let shell_command = command
        .iter()
        .map(String::as_str)
        .chain([toggle_overlay])
        .map(quote_for_shell)
        .collect::<Vec<_>>()
        .join(" ");

    if desktop.contains("niri") {
        let arguments = command
            .iter()
            .map(String::as_str)
            .chain([toggle_overlay])
            .map(|argument| format!("\"{argument}\""))
            .collect::<Vec<_>>()
            .join(" ");
        let shortcut = combination
            .modifiers
            .iter()
            .copied()
            .chain([key.as_str()])
            .collect::<Vec<_>>()
            .join("+");
        return Some(CompositorBinding {
            compositor: "niri",
            line: format!("{shortcut} {{ spawn {arguments}; }}"),
        });
    }
    if desktop.contains("hyprland") {
        let modifiers = combination.modifiers.join(" ").to_uppercase();
        return Some(CompositorBinding {
            compositor: "Hyprland",
            line: format!("bind = {modifiers}, {key}, exec, {shell_command}"),
        });
    }
    if desktop.contains("sway") {
        let shortcut = combination
            .modifiers
            .iter()
            .copied()
            .chain([combination.key.as_str()])
            .collect::<Vec<_>>()
            .join("+");
        return Some(CompositorBinding {
            compositor: "sway",
            line: format!("bindsym {shortcut} exec {shell_command}"),
        });
    }
    None
}

fn quote_for_shell(argument: &str) -> String {
    if argument.contains(char::is_whitespace) {
        format!("'{argument}'")
    } else {
        argument.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctrl_alt_l() -> Combination {
        Combination {
            modifiers: vec!["Ctrl", "Alt"],
            key: "l".to_owned(),
        }
    }

    fn command() -> Vec<String> {
        vec!["quadra-rewrite".to_owned()]
    }

    #[test]
    fn writes_a_niri_bind() {
        let binding = binding_for("niri", &ctrl_alt_l(), &command()).unwrap();
        assert_eq!(
            binding.line,
            "Ctrl+Alt+L { spawn \"quadra-rewrite\" \"--toggle-overlay\"; }"
        );
    }

    #[test]
    fn writes_a_hyprland_bind() {
        let binding = binding_for("Hyprland", &ctrl_alt_l(), &command()).unwrap();
        assert_eq!(
            binding.line,
            "bind = CTRL ALT, L, exec, quadra-rewrite --toggle-overlay"
        );
    }

    #[test]
    fn writes_a_sway_bind() {
        let binding = binding_for("sway", &ctrl_alt_l(), &command()).unwrap();
        assert_eq!(
            binding.line,
            "bindsym Ctrl+Alt+l exec quadra-rewrite --toggle-overlay"
        );
    }

    #[test]
    fn flatpak_arguments_stay_separate_in_niri_and_joined_in_shell_commands() {
        let flatpak: Vec<String> = ["flatpak", "run", "org.quadrahq.Quadra"]
            .map(str::to_owned)
            .into();
        let niri = binding_for("niri", &ctrl_alt_l(), &flatpak).unwrap();
        assert!(
            niri.line
                .contains("\"flatpak\" \"run\" \"org.quadrahq.Quadra\" \"--toggle-overlay\"")
        );
        let sway = binding_for("sway", &ctrl_alt_l(), &flatpak).unwrap();
        assert!(
            sway.line
                .ends_with("exec flatpak run org.quadrahq.Quadra --toggle-overlay")
        );
    }

    #[test]
    fn paths_with_spaces_are_quoted_for_shell_commands() {
        let command = vec!["/my apps/Quadra.AppImage".to_owned()];
        let binding = binding_for("sway", &ctrl_alt_l(), &command).unwrap();
        assert!(
            binding
                .line
                .contains("'/my apps/Quadra.AppImage' --toggle-overlay")
        );
    }

    #[test]
    fn other_desktops_have_no_bind_to_suggest() {
        assert_eq!(binding_for("KDE", &ctrl_alt_l(), &command()), None);
    }
}
