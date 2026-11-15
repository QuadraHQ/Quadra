use super::custom_flags::custom_flags_group;
use super::fast_flag_allowlist::{
    ALLOWLIST, ALLOWLIST_SOURCE, AllowlistedFlag, FlagCategory, FlagKind,
};
use crate::state::{AppState, Choice, FastFlagValue};
use adw::prelude::*;
use gtk::glib;

const INVALID_NUMBER_STYLE: &str = "error";

#[derive(Clone, Copy, PartialEq, Eq)]
enum ToggleSetting {
    Default,
    On,
    Off,
}

impl Choice for ToggleSetting {
    const ALL: &'static [Self] = &[Self::Default, Self::On, Self::Off];

    fn label(self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::On => "On",
            Self::Off => "Off",
        }
    }
}

impl ToggleSetting {
    fn of(value: Option<FastFlagValue>) -> Self {
        match value {
            Some(FastFlagValue::Toggle(true)) => Self::On,
            Some(FastFlagValue::Toggle(false)) => Self::Off,
            _ => Self::Default,
        }
    }

    fn value(self) -> Option<FastFlagValue> {
        match self {
            Self::Default => None,
            Self::On => Some(FastFlagValue::Toggle(true)),
            Self::Off => Some(FastFlagValue::Toggle(false)),
        }
    }
}

struct PerformancePreset {
    name: &'static str,
    description: &'static str,
    flags: &'static [(&'static str, FastFlagValue)],
}

const PERFORMANCE_PRESETS: &[PerformancePreset] = &[
    PerformancePreset {
        name: "Maximum Performance",
        description: "All performance flags enabled. Lowest visual quality, highest FPS.",
        flags: &[
            ("FFlagDebugDisablePostFx", FastFlagValue::Toggle(true)),
            ("FFlagDebugDisableClusterCulling", FastFlagValue::Toggle(true)),
            ("FFlagDebugGraphicsDisableComputeShaders", FastFlagValue::Toggle(true)),
            ("FFlagTaskSchedulerLimitFrequency", FastFlagValue::Toggle(false)),
            ("DFIntTaskSchedulerTargetFps", FastFlagValue::Number(999)),
            ("FIntRenderMaxDrawCallsPerFrame", FastFlagValue::Number(500)),
            ("FFlagRenderFontResolution", FastFlagValue::Number(1)),
            ("FFlagDebugGraphicsPreferOpenGL", FastFlagValue::Toggle(true)),
            ("FFlagDebugGraphicsPreferVulkan", FastFlagValue::Toggle(true)),
            ("DFIntGCJobFrequencyMs", FastFlagValue::Number(250)),
            ("DFIntPhysicsStepsPerFrame", FastFlagValue::Number(1)),
            ("FFlagLuaAppEnableLowMemoryMode", FastFlagValue::Toggle(true)),
            ("FFlagDisableShadows", FastFlagValue::Toggle(true)),
            ("FFlagDisableBloom", FastFlagValue::Toggle(true)),
            ("FFlagDisableDepthOfField", FastFlagValue::Toggle(true)),
            ("FFlagDisableGlobalShadows", FastFlagValue::Toggle(true)),
            ("DFIntConnectionMTUSize", FastFlagValue::Number(1400)),
            ("FFlagEnableReducedLatency", FastFlagValue::Toggle(true)),
        ],
    },
    PerformancePreset {
        name: "Balanced",
        description: "Moderate performance gains with acceptable visuals.",
        flags: &[
            ("FFlagDebugDisablePostFx", FastFlagValue::Toggle(true)),
            ("FFlagTaskSchedulerLimitFrequency", FastFlagValue::Toggle(false)),
            ("DFIntTaskSchedulerTargetFps", FastFlagValue::Number(144)),
            ("DFIntGCJobFrequencyMs", FastFlagValue::Number(250)),
            ("DFIntPhysicsStepsPerFrame", FastFlagValue::Number(1)),
            ("FFlagEnableReducedLatency", FastFlagValue::Toggle(true)),
        ],
    },
    PerformancePreset {
        name: "Low Latency / Anti-Stutter",
        description: "Optimized for reducing lag spikes and input latency. Keeps visuals decent.",
        flags: &[
            ("FFlagDebugDisablePostFx", FastFlagValue::Toggle(true)),
            ("FFlagTaskSchedulerLimitFrequency", FastFlagValue::Toggle(false)),
            ("DFIntTaskSchedulerTargetFps", FastFlagValue::Number(144)),
            ("DFIntGCJobFrequencyMs", FastFlagValue::Number(250)),
            ("DFIntPhysicsStepsPerFrame", FastFlagValue::Number(1)),
            ("FFlagEnableReducedLatency", FastFlagValue::Toggle(true)),
            ("DFIntConnectionMTUSize", FastFlagValue::Number(1400)),
        ],
    },
    PerformancePreset {
        name: "Reset to Defaults",
        description: "Clears all performance flags, uses Roblox defaults.",
        flags: &[
            ("FFlagDebugDisablePostFx", FastFlagValue::Toggle(false)),
            ("FFlagDebugDisableClusterCulling", FastFlagValue::Toggle(false)),
            ("FFlagDebugGraphicsDisableComputeShaders", FastFlagValue::Toggle(false)),
            ("FFlagTaskSchedulerLimitFrequency", FastFlagValue::Toggle(true)),
            ("DFIntTaskSchedulerTargetFps", FastFlagValue::Number(60)),
            ("FIntRenderMaxDrawCallsPerFrame", FastFlagValue::Number(0)),
            ("FFlagRenderFontResolution", FastFlagValue::Number(0)),
            ("FFlagDebugGraphicsPreferOpenGL", FastFlagValue::Toggle(false)),
            ("FFlagDebugGraphicsPreferVulkan", FastFlagValue::Toggle(false)),
            ("DFIntGCJobFrequencyMs", FastFlagValue::Number(0)),
            ("DFIntPhysicsStepsPerFrame", FastFlagValue::Number(0)),
            ("FFlagLuaAppEnableLowMemoryMode", FastFlagValue::Toggle(false)),
            ("FFlagDisableShadows", FastFlagValue::Toggle(false)),
            ("FFlagDisableBloom", FastFlagValue::Toggle(false)),
            ("FFlagDisableDepthOfField", FastFlagValue::Toggle(false)),
            ("FFlagDisableGlobalShadows", FastFlagValue::Toggle(false)),
            ("DFIntConnectionMTUSize", FastFlagValue::Number(0)),
            ("FFlagEnableReducedLatency", FastFlagValue::Toggle(false)),
        ],
    },
];

pub fn fast_flags_section(state: &AppState) -> gtk::Box {
    let allowlist_link = gtk::LinkButton::builder()
        .uri(ALLOWLIST_SOURCE)
        .label("View the allowlist")
        .valign(gtk::Align::Center)
        .build();

    let about_group = adw::PreferencesGroup::builder()
        .title("FastFlags")
        .description(
            "Roblox only applies flags on its allowlist. Quadra writes the flags you set to ClientAppSettings.json each time Roblox starts.",
        )
        .header_suffix(&allowlist_link)
        .build();

    // Performance presets
    let presets_group = adw::PreferencesGroup::builder()
        .title("Performance Presets")
        .description("Quick-apply flag combinations for FPS gains. These are client-side only and not bannable.")
        .build();

    for preset in PERFORMANCE_PRESETS {
        let button = gtk::Button::builder()
            .label(preset.name)
            .tooltip_text(preset.description)
            .halign(gtk::Align::Start)
            .build();
        let state_clone = state.clone();
        let flags = preset.flags;
        button.connect_clicked(move |_| {
            apply_preset(&state_clone, flags);
        });
        presets_group.add(&button);
    }

    let section = gtk::Box::new(gtk::Orientation::Vertical, 12);
    section.append(&about_group);
    section.append(&presets_group);
    for category in ALLOWLIST {
        section.append(&category_group(state, category));
    }
    section.append(&custom_flags_group(state));

    section
}

fn apply_preset(state: &AppState, flags: &[(&str, FastFlagValue)]) {
    let mut new_flags = state.fast_flags().clone();
    for (name, value) in flags {
        new_flags = new_flags.with(name, Some(value.clone()));
    }
    state.set_fast_flags(new_flags);
}

fn category_group(state: &AppState, category: &FlagCategory) -> adw::PreferencesGroup {
    let group = adw::PreferencesGroup::builder()
        .title(category.title)
        .build();

    for flag in category.flags {
        match flag.kind {
            FlagKind::Toggle => group.add(&toggle_row(state, flag)),
            FlagKind::Number => group.add(&number_row(state, flag)),
        }
    }

    group
}

fn toggle_row(state: &AppState, flag: &'static AllowlistedFlag) -> adw::ComboRow {
    let current_setting = ToggleSetting::of(state.fast_flags().get(flag.name));

    let row = adw::ComboRow::builder()
        .title(flag.title)
        .subtitle(flag.name)
        .model(&gtk::StringList::new(&ToggleSetting::labels()))
        .selected(current_setting.position())
        .build();

    row.connect_selected_notify(glib::clone!(
        #[strong]
        state,
        move |row| {
            let setting =
                ToggleSetting::at_position(row.selected()).unwrap_or(ToggleSetting::Default);
            set_flag(&state, flag.name, setting.value());
        }
    ));
    state.connect_notify_local(
        Some("fast-flags"),
        glib::clone!(
            #[weak]
            row,
            move |state, _| {
                let setting = ToggleSetting::of(state.fast_flags().get(flag.name));
                if row.selected() != setting.position() {
                    row.set_selected(setting.position());
                }
            }
        ),
    );

    row
}

fn number_row(state: &AppState, flag: &'static AllowlistedFlag) -> adw::ActionRow {
    let current_number = match state.fast_flags().get(flag.name) {
        Some(FastFlagValue::Number(number)) => number.to_string(),
        _ => String::new(),
    };

    let entry = gtk::Entry::builder()
        .text(current_number)
        .placeholder_text("Default")
        .input_purpose(gtk::InputPurpose::Number)
        .width_chars(8)
        .xalign(1.0)
        .valign(gtk::Align::Center)
        .build();

    let row = adw::ActionRow::builder()
        .title(flag.title)
        .subtitle(flag.name)
        .activatable_widget(&entry)
        .build();
    row.add_suffix(&entry);

    entry.connect_changed(glib::clone!(
        #[strong]
        state,
        move |entry| {
            let text = entry.text();
            let text = text.trim();
            if text.is_empty() {
                show_number_is_valid(entry, true);
                set_flag(&state, flag.name, None);
            } else if let Ok(number) = text.parse() {
                show_number_is_valid(entry, true);
                set_flag(&state, flag.name, Some(FastFlagValue::Number(number)));
            } else {
                show_number_is_valid(entry, false);
            }
        }
    ));
    state.connect_notify_local(
        Some("fast-flags"),
        glib::clone!(
            #[weak]
            entry,
            move |state, _| show_number_from_state(&entry, flag, state)
        ),
    );

    row
}

fn show_number_from_state(entry: &gtk::Entry, flag: &AllowlistedFlag, state: &AppState) {
    let typed = entry.text();
    let typed = typed.trim();
    let typed_number = typed.parse::<i64>().ok();
    if !typed.is_empty() && typed_number.is_none() {
        return;
    }

    let saved_number = match state.fast_flags().get(flag.name) {
        Some(FastFlagValue::Number(number)) => Some(number),
        _ => None,
    };
    if typed_number != saved_number {
        entry.set_text(
            &saved_number
                .map(|number| number.to_string())
                .unwrap_or_default(),
        );
    }
}

fn show_number_is_valid(entry: &gtk::Entry, is_valid: bool) {
    if is_valid {
        entry.remove_css_class(INVALID_NUMBER_STYLE);
        entry.set_tooltip_text(None);
    } else {
        entry.add_css_class(INVALID_NUMBER_STYLE);
        entry.set_tooltip_text(Some(
            "Enter a whole number, or leave it empty for the default",
        ));
    }
}

fn set_flag(state: &AppState, name: &str, value: Option<FastFlagValue>) {
    state.set_fast_flags(state.fast_flags().with(name, value));
}
