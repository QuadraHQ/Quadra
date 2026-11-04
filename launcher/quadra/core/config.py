"""The launcher's settings: their defaults, loading and saving them, and the last error."""

import json

from ..darling.runtime import is_bundled
from . import files, locations

__all__ = [
    "DEFAULT_SETTINGS",
    "TRACE_ENV",
    "TRACE_ENV_INHERITED",
    "load_settings",
    "save_settings",
    "clear_last_error",
    "format_playtime",
]

DEFAULT_SETTINGS = {
    "setup_complete": False,
    "language": "en",
    "launcher_appearance": "gnome",
    "mouse_sensitivity": 1.0,
    "scroll_sensitivity": 1.5,
    "auto_patch_throttle": True,
    "raw_mouse": True,
    "display_backend": "x11",
    "dpi_scale": 1.0,
    "renderer": "opengl",
    "metal_shader_preload": True,
    "metal_library_preload": True,
    "metal_resource_preload": True,
    "metal_diagnostics": False,
    # Quadra M2VK tuning (graphics.metal_tuning_environment); the values are the defaults.
    "metal_accuracy": "fast",
    "metal_present_mode": "auto",
    "metal_memory_priority": True,
    "metal_compile_threads": 0,
    "metal_warm_states": 256,
    "metal_vram_limit_gib": 0,
    # Play with OpenGL this once, rather than not at all, when Quadra M2VK cannot start.
    "metal_fallback": True,
    "gpu": "auto",
    "roblox_channel": "",
    "mangohud": False,
    # VR through the host's OpenXR runtime (see vr.py); experimental.
    "vr": False,
    "vr_render_scale": 70,
    "vr_mirror": True,
    # Set from the warning shown before VR is first turned on ("Don't show this again").
    "vr_warning_hidden": False,
    "hide_menu_bar": False,
    "dns": "system",
    "dns_custom": "",
    "show_launcher_after_exit": True,  # legacy: replaced by open_settings_after_exit
    # Quadra starts Roblox directly behind a small splash; the settings window
    # opens from the tray icon, the shortcut's "Settings" action or --settings.
    "open_settings_after_exit": False,
    "tray_icon": True,
    "diagnostic_signals": False,
    "trace_udp": False,
    "trace_lock": False,
    "trace_events": False,
    "trace_gl": False,
    "fps_log": False,
    "trace_keys": False,
    "keep_logs": 30,
    "show_sidebar": True,
    "framerate_cap": 0,
    "hide_launcher_on_launch": True,
    "show_playtime": True,
    # Sends the game server's IP address to RoValra, so it is opt-in. (The
    # former show_server_location key was saved as true for everyone.)
    "server_location_lookup": False,
    # A desktop notification with the server's location when you join one.
    # Only fires while the lookup above is on.
    "server_location_notify": True,
    # Answer Roblox's telemetry hosts with NXDOMAIN. Needs a DNS provider
    # other than "system" (the forwarder is what answers).
    "block_telemetry": False,
    # Join servers in a chosen region (RoValra's server list). Opt-in: RoValra
    # sees the place id and the region.
    # Limit Roblox to this many CPU cores (0 = no limit). Experimental.
    "roblox_cpu_limit": 0,
    # Which macOS translation layer runs Roblox: "rosemary" (the default for a new install) or "darling"
    # (outdated). load_settings keeps an install that predates the choice, and a bundled one, on Darling.
    "runtime_backend": "rosemary",
    "setup_darling_variant": "",
    "rosemary_program": "",
    "rosemary_runners": [],
    # Which Rosemary Quadra installs: "quadra" (its own build, adds VR) or "upstream" (the regular one).
    "rosemary_source": "quadra",
    "rosemary_prefix": "",
    "rosemary_clear_cache_on_launch": False,
    "rosemary_engine": "auto",
    "rosemary_force_opengl": False,
    "rosemary_video_driver": "",
    "rosemary_swap_interval": "",
    "rosemary_process_server": True,
    "rosemary_presentation_mode": "",
    "rosemary_refresh_rate": "",
    "rosemary_decorations": "",
    "rosemary_menu": "",
    "rosemary_command_key": "",
    "rosemary_confine_pointer": False,
    "rosemary_control_edit_keys": False,
    "rosemary_plain_ctrl_click": False,
    # Key of an installed build to stay on (a rollback); empty follows Roblox's latest.
    "rosemary_pinned_build": "",
    # VR on Rosemary: run Roblox's OpenGL through Mesa's zink (on Vulkan) instead of the driver's own GL.
    "rosemary_vr_zink": False,
    "region_join": False,
    "region_preferred": "",
    # A toast when the game you joined was updated in the last day.
    "game_update_notice": True,
    "playtime_seconds": 0,
    "discord_rpc": True,
    "discord_rpc_game": True,
    "discord_rpc_icon": False,
    "discord_rpc_time": True,
    "auto_check_roblox_updates": True,
    "auto_check_ciderdarling_updates": True,
    "mod_death_sound": "default",
    "mod_custom_death_sound": "",
    "mod_old_character_sounds": False,
    "mod_cursor_type": "default",
    "mod_custom_cursor": "",
    "mod_custom_font": "",
    "enable_custom_mods": True,
    # Lets diagnostics read the game's thread stacks under Yama ptrace_scope=1.
    "allow_debugger": False,
    "trace_text_input": False,
    # CiderDarling only (Settings -> CiderDarling); see ciderdarling_variables.
    "cider_trace": False,
    "cider_verbose_server_log": False,
    "cider_thread_kicks": False,
    # Milliseconds between looks at the game's idle I/O loops (QUADRA_KEVENT_POLL_MS in net_trace.c). Without it
    # the menu waited about 18 s after leaving a game for a loop that was never woken; 0 turns it off.
    "kevent_poll_ms": 100,
    # Roblox Studio (studio.py), through Wine. See Settings -> Studio.
    "studio_renderer": "dxvk",  # dxvk, dxvk-sarek, d3d11 (Wine's OpenGL), d3d11fl10 or vulkan
    "studio_gpu": "",  # PCI address of the GPU Studio renders on; "" is the system default
    "studio_gamemode": True,
    "studio_webview": True,  # Edge WebView2, which Studio's sign-in and web panels need
    "studio_discord": True,
    "studio_theme": True,  # follow the system's light/dark preference
    "studio_backup": True,  # keep Studio's sign-in and settings outside the Wine prefix
    "studio_wine": "",  # a Kombucha release tag; "" is the newest stable
    "studio_wine_root": "",  # a Wine folder of your own (it holds bin/wine); replaces Kombucha
    "studio_version": "",  # a Studio deployment (version-...) to stay on; "" is the newest
    "studio_channel": "",  # a deployment channel; "" is the one of the signed-in account
    "studio_fflags": {},  # extra Studio fast flags
}

# Settings -> environment variables understood by the shim.
TRACE_ENV = {
    "diagnostic_signals": "QUADRA_DIAGNOSTIC_SIGNALS",
    "trace_udp": "QUADRA_TRACE_UDP",
    "trace_lock": "QUADRA_TRACE_LOCK",
    "trace_events": "QUADRA_TRACE_EVENTS",
    "trace_gl": "QUADRA_TRACE_GL",
    "fps_log": "QUADRA_FPS_LOG",
    "trace_keys": "QUADRA_TRACE_KEYS",
    "allow_debugger": "QUADRA_ALLOW_DEBUGGER",
}
# These also apply when set in the launcher's own environment.
TRACE_ENV_INHERITED = ("diagnostic_signals", "allow_debugger")


def load_settings():
    """Settings with defaults for anything missing. A stored value of the
    wrong type (a hand edit, a damaged file) falls back to its default rather
    than failing every launch later."""
    settings = dict(DEFAULT_SETTINGS)
    try:
        stored = json.loads(locations.SETTINGS_FILE.read_text())
    except (OSError, ValueError):
        stored = {}
    if not isinstance(stored, dict):
        stored = {}
    # Rosemary is the default for a new install only. A settings file with no runtime in it was saved by an
    # install that ran Darling, and a Flatpak or Nix package carries Darling inside it; neither is moved.
    if "runtime_backend" not in stored and (stored or is_bundled()):
        settings["runtime_backend"] = "darling"
    for key, value in stored.items():
        if key == "launcher_appearance":
            settings[key] = (
                value
                if isinstance(value, str) and value in ("grid", "gnome", "monochrome")
                else DEFAULT_SETTINGS[key]
            )
            continue
        if key == "dpi_scale":
            from ..display import validated_dpi_scale

            settings[key] = validated_dpi_scale(value)
            continue
        default = DEFAULT_SETTINGS.get(key)
        if isinstance(default, bool):
            valid = isinstance(value, bool)
        elif isinstance(default, (int, float)):
            valid = isinstance(value, (int, float)) and not isinstance(value, bool)
            value = type(default)(value) if valid else value
        else:
            valid = default is None or isinstance(value, type(default))
        if valid:
            settings[key] = value
    return settings


def save_settings(settings):
    files._write_atomically(locations.SETTINGS_FILE, json.dumps(dict(settings), indent=2, ensure_ascii=False))


def clear_last_error():
    """Forget the last error once an install or launch has succeeded."""
    try:
        locations.LAST_ERROR.unlink(missing_ok=True)
    except OSError:
        pass


def format_playtime(seconds: int) -> str:
    """Format playtime in seconds into a concise string like '1M', '2H 15M'."""
    seconds = max(0, int(seconds))
    total_mins = seconds // 60
    if total_mins < 60:
        return f"{max(1, total_mins)}M" if seconds >= 60 else "0M"
    hours = total_mins // 60
    mins = total_mins % 60
    if mins:
        return f"{hours}H {mins}M"
    return f"{hours}H"
