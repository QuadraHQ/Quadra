"""What each runtime (macOS translation layer) can do in Quadra.

The settings pages are the same for both; a setting that only one runtime
reads is greyed out and tagged when the other is chosen, so nothing silently
does nothing. A feature is added to a runtime here once it is implemented."""

DARLING = "darling"
ROSEMARY = "rosemary"

FEATURES = {
    DARLING: {"mods", "dns", "telemetry", "cpu_limit", "renderer", "m2vk", "vr", "input", "throttle",
              "client_updates", "diagnostics", "cider", "web_signin", "mangohud", "menu_bar"},
    # Rosemary has its own gamepad (SDL3) and Metal translation. VR needs Quadra's
    # build of Rosemary (rosemary_xr.supports_vr); the VR page says so.
    ROSEMARY: {"mods", "cpu_limit", "mangohud", "rosemary", "menu_bar", "vr"},
}

NAMES = {DARLING: "Darling", ROSEMARY: "Rosemary"}


def supports(runtime, feature):
    return feature in FEATURES.get(runtime, ())


def other_runtimes(feature):
    """Names of the runtimes that read a feature, for the "<name> only" tag."""
    return [NAMES[r] for r, features in FEATURES.items() if feature in features]
