#[derive(Clone, Copy)]
pub enum FlagKind {
    Toggle,
    Number,
}

pub struct AllowlistedFlag {
    pub name: &'static str,
    pub title: &'static str,
    pub kind: FlagKind,
}

pub struct FlagCategory {
    pub title: &'static str,
    pub flags: &'static [AllowlistedFlag],
}

const fn toggle(name: &'static str, title: &'static str) -> AllowlistedFlag {
    AllowlistedFlag {
        name,
        title,
        kind: FlagKind::Toggle,
    }
}

const fn number(name: &'static str, title: &'static str) -> AllowlistedFlag {
    AllowlistedFlag {
        name,
        title,
        kind: FlagKind::Number,
    }
}

pub const ALLOWLIST_SOURCE: &str =
    "https://devforum.roblox.com/t/allowlist-for-local-client-configuration-via-fast-flags/3966569";

pub const ALLOWLIST: &[FlagCategory] = &[
    FlagCategory {
        title: "Geometry",
        flags: &[
            number(
                "DFIntCSGLevelOfDetailSwitchingDistance",
                "Level of detail switching distance",
            ),
            number(
                "DFIntCSGLevelOfDetailSwitchingDistanceL12",
                "Level 1 to 2 switching distance",
            ),
            number(
                "DFIntCSGLevelOfDetailSwitchingDistanceL23",
                "Level 2 to 3 switching distance",
            ),
            number(
                "DFIntCSGLevelOfDetailSwitchingDistanceL34",
                "Level 3 to 4 switching distance",
            ),
        ],
    },
    FlagCategory {
        title: "Rendering",
        flags: &[
            toggle(
                "FFlagHandleAltEnterFullscreenManually",
                "Handle Alt+Enter fullscreen manually",
            ),
            toggle(
                "DFFlagTextureQualityOverrideEnabled",
                "Override texture quality",
            ),
            number("DFIntTextureQualityOverride", "Texture quality"),
            number("FIntDebugForceMSAASamples", "Antialiasing samples"),
            toggle("DFFlagDisableDPIScale", "Disable DPI scaling"),
            toggle("FFlagDebugSkyGray", "Gray sky"),
            toggle("DFFlagDebugPauseVoxelizer", "Pause the voxelizer"),
            number(
                "DFIntDebugFRMQualityLevelOverride",
                "Graphics quality level",
            ),
            number("FIntFRMMaxGrassDistance", "Maximum grass distance"),
            number("FIntFRMMinGrassDistance", "Minimum grass distance"),
        ],
    },
    FlagCategory {
        title: "User interface",
        flags: &[number(
            "FIntGrassMovementReducedMotionFactor",
            "Grass movement with reduced motion",
        )],
    },
];
