use super::bound_rows::choice;
use super::custom_flags::custom_flags_group;
use super::fast_flag_allowlist::{
    ALLOWLIST, ALLOWLIST_SOURCE, AllowlistedFlag, FlagCategory, FlagKind,
};
use super::{Settings, SettingsMessage};
use crate::components::preferences::{
    PreferenceRow, preference_group, preference_row, push_button, with_tooltip,
};
use crate::components::style::{self, Emphasis, Look};
use crate::quadra::{Message, act};
use crate::state::{AppState, Choice, FastFlagValue, FastFlags};
use iced::Element;
use iced::widget::{pick_list, text_input};
use std::collections::HashMap;

const NUMBER_INPUT_WIDTH: f32 = 110.0;
const CUSTOM_PRESET_LABEL: &str = "Custom";
const CUSTOM_PRESET_DESCRIPTION: &str = "Your own mix of flags, set below.";

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
            (
                "FFlagDebugDisableClusterCulling",
                FastFlagValue::Toggle(true),
            ),
            (
                "FFlagDebugGraphicsDisableComputeShaders",
                FastFlagValue::Toggle(true),
            ),
            (
                "FFlagTaskSchedulerLimitFrequency",
                FastFlagValue::Toggle(false),
            ),
            ("DFIntTaskSchedulerTargetFps", FastFlagValue::Number(999)),
            ("FIntRenderMaxDrawCallsPerFrame", FastFlagValue::Number(500)),
            ("FFlagRenderFontResolution", FastFlagValue::Number(1)),
            (
                "FFlagDebugGraphicsPreferOpenGL",
                FastFlagValue::Toggle(true),
            ),
            (
                "FFlagDebugGraphicsPreferVulkan",
                FastFlagValue::Toggle(true),
            ),
            ("DFIntGCJobFrequencyMs", FastFlagValue::Number(250)),
            ("DFIntPhysicsStepsPerFrame", FastFlagValue::Number(1)),
            (
                "FFlagLuaAppEnableLowMemoryMode",
                FastFlagValue::Toggle(true),
            ),
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
            (
                "FFlagTaskSchedulerLimitFrequency",
                FastFlagValue::Toggle(false),
            ),
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
            (
                "FFlagTaskSchedulerLimitFrequency",
                FastFlagValue::Toggle(false),
            ),
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
            (
                "FFlagDebugDisableClusterCulling",
                FastFlagValue::Toggle(false),
            ),
            (
                "FFlagDebugGraphicsDisableComputeShaders",
                FastFlagValue::Toggle(false),
            ),
            (
                "FFlagTaskSchedulerLimitFrequency",
                FastFlagValue::Toggle(true),
            ),
            ("DFIntTaskSchedulerTargetFps", FastFlagValue::Number(60)),
            ("FIntRenderMaxDrawCallsPerFrame", FastFlagValue::Number(0)),
            ("FFlagRenderFontResolution", FastFlagValue::Number(0)),
            (
                "FFlagDebugGraphicsPreferOpenGL",
                FastFlagValue::Toggle(false),
            ),
            (
                "FFlagDebugGraphicsPreferVulkan",
                FastFlagValue::Toggle(false),
            ),
            ("DFIntGCJobFrequencyMs", FastFlagValue::Number(0)),
            ("DFIntPhysicsStepsPerFrame", FastFlagValue::Number(0)),
            (
                "FFlagLuaAppEnableLowMemoryMode",
                FastFlagValue::Toggle(false),
            ),
            ("FFlagDisableShadows", FastFlagValue::Toggle(false)),
            ("FFlagDisableBloom", FastFlagValue::Toggle(false)),
            ("FFlagDisableDepthOfField", FastFlagValue::Toggle(false)),
            ("FFlagDisableGlobalShadows", FastFlagValue::Toggle(false)),
            ("DFIntConnectionMTUSize", FastFlagValue::Number(0)),
            ("FFlagEnableReducedLatency", FastFlagValue::Toggle(false)),
        ],
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresetChoice {
    Custom,
    Preset(usize),
}

impl PresetChoice {
    const ALL: [Self; 5] = [
        Self::Custom,
        Self::Preset(0),
        Self::Preset(1),
        Self::Preset(2),
        Self::Preset(3),
    ];

    fn preset(self) -> Option<&'static PerformancePreset> {
        match self {
            Self::Custom => None,
            Self::Preset(position) => PERFORMANCE_PRESETS.get(position),
        }
    }

    fn description(self) -> &'static str {
        self.preset()
            .map_or(CUSTOM_PRESET_DESCRIPTION, |preset| preset.description)
    }
}

impl std::fmt::Display for PresetChoice {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.preset().map_or(CUSTOM_PRESET_LABEL, |preset| preset.name))
    }
}

pub struct ChosenPreset {
    choice: PresetChoice,
    flags_when_chosen: FastFlags,
}

pub fn choose_preset(state: &AppState, choice: PresetChoice) -> ChosenPreset {
    if let Some(preset) = choice.preset() {
        let flags = state.fast_flags();
        if !is_applied(preset, &flags) {
            let applied = preset
                .flags
                .iter()
                .fold(flags, |flags, (name, value)| flags.with(name, Some(value.clone())));
            state.set_fast_flags(applied);
        }
    }
    ChosenPreset {
        choice,
        flags_when_chosen: state.fast_flags(),
    }
}

fn shown_preset(chosen: Option<&ChosenPreset>, flags: &FastFlags) -> PresetChoice {
    if let Some(chosen) = chosen {
        let flags_are_unchanged = chosen.flags_when_chosen == *flags;
        let preset_is_still_applied = chosen
            .choice
            .preset()
            .is_some_and(|preset| is_applied(preset, flags));
        if flags_are_unchanged || preset_is_still_applied {
            return chosen.choice;
        }
    }

    PERFORMANCE_PRESETS
        .iter()
        .enumerate()
        .filter(|(_, preset)| is_applied(preset, flags))
        .max_by_key(|(_, preset)| preset.flags.len())
        .map_or(PresetChoice::Custom, |(position, _)| {
            PresetChoice::Preset(position)
        })
}

fn is_applied(preset: &PerformancePreset, flags: &FastFlags) -> bool {
    preset
        .flags
        .iter()
        .all(|(name, value)| flags.get(name).as_ref() == Some(value))
}

fn presets_group<'a>(
    chosen: Option<&ChosenPreset>,
    flags: &FastFlags,
    look: Look,
) -> Element<'a, Message> {
    let shown = shown_preset(chosen, flags);
    let picker = pick_list(PresetChoice::ALL, Some(shown), |choice| {
        Message::Settings(SettingsMessage::PresetChosen(choice))
    })
    .font(look.font)
    .text_size(14)
    .style(style::choice(look))
    .menu_style(style::menu(look));

    preference_group(look)
        .title("Performance Presets")
        .description(
            "Quick-apply flag combinations for FPS gains. These are client-side only and not bannable.",
        )
        .push(
            preference_row(look, "Preset")
                .subtitle(shown.description())
                .suffix(picker),
        )
        .into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

pub fn fast_flags_section<'a>(
    settings: &'a Settings,
    state: &AppState,
    look: Look,
) -> Vec<Element<'a, Message>> {
    let about_group = preference_group(look)
        .title("FastFlags")
        .description(
            "Roblox only applies flags on its allowlist. Quadra writes the flags you set to ClientAppSettings.json each time Roblox starts.",
        )
        .header_suffix(
            push_button(look, "View the allowlist", Emphasis::Flat)
                .on_press(Message::Open(ALLOWLIST_SOURCE.to_owned())),
        );

    let flags = state.fast_flags();
    let mut sections = vec![
        about_group.into(),
        presets_group(settings.chosen_preset.as_ref(), &flags, look),
    ];
    sections.extend(
        ALLOWLIST
            .iter()
            .map(|category| category_group(settings, &flags, category, look)),
    );
    sections.push(custom_flags_group(&settings.custom_flags, look));
    sections
}

fn category_group<'a>(
    settings: &'a Settings,
    flags: &FastFlags,
    category: &'static FlagCategory,
    look: Look,
) -> Element<'a, Message> {
    let rows = category.flags.iter().map(|flag| match flag.kind {
        FlagKind::Toggle => toggle_row(flags.get(flag.name), flag, look),
        FlagKind::Number => number_row(settings, flags.get(flag.name), flag, look),
    });
    preference_group(look)
        .title(category.title)
        .extend(rows)
        .into()
}

fn toggle_row<'a>(
    value: Option<FastFlagValue>,
    flag: &'static AllowlistedFlag,
    look: Look,
) -> PreferenceRow<'a> {
    preference_row(look, flag.title)
        .monospace_subtitle(flag.name)
        .suffix(choice(
            look,
            ToggleSetting::of(value),
            true,
            move |setting| act(move |state| set_flag(state, flag.name, setting.value())),
        ))
}

fn number_row<'a>(
    settings: &'a Settings,
    value: Option<FastFlagValue>,
    flag: &'static AllowlistedFlag,
    look: Look,
) -> PreferenceRow<'a> {
    let invalid = settings.invalid_numbers.get(flag.name);
    let shown = match (invalid, value) {
        (Some(typed), _) => typed.clone(),
        (None, Some(FastFlagValue::Number(number))) => number.to_string(),
        (None, _) => String::new(),
    };

    let input = text_input("Default", &shown)
        .font(look.mono)
        .size(13)
        .width(NUMBER_INPUT_WIDTH)
        .align_x(iced::Alignment::End)
        .padding([6, 10])
        .style(style::input(look, invalid.is_none()))
        .on_input(move |text| {
            Message::Settings(SettingsMessage::NumberFlagChanged(flag.name, text))
        });
    let input = with_tooltip(
        look,
        input,
        "Enter a whole number, or leave it empty for the default",
    );

    preference_row(look, flag.title)
        .monospace_subtitle(flag.name)
        .suffix(input)
}

pub fn number_changed(
    state: &AppState,
    invalid_numbers: &mut HashMap<&'static str, String>,
    name: &'static str,
    text: String,
) {
    let typed = text.trim();
    if typed.is_empty() {
        invalid_numbers.remove(name);
        set_flag(state, name, None);
    } else if let Ok(number) = typed.parse() {
        invalid_numbers.remove(name);
        set_flag(state, name, Some(FastFlagValue::Number(number)));
    } else {
        invalid_numbers.insert(name, text);
    }
}

fn set_flag(state: &AppState, name: &str, value: Option<FastFlagValue>) {
    state.set_fast_flags(state.fast_flags().with(name, value));
}
