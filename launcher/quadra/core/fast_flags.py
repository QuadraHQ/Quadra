"""Fast flags: the user's own, the Roblox bundle's file, and the ones Darling needs."""

import json

from . import files, locations

__all__ = [
    "DARLING_FLAGS",
    "FAST_FLAGS_USER",
    "_read_flags",
    "load_bundle_flags",
    "save_bundle_flags",
    "load_fast_flags",
    "save_fast_flags",
    "parse_flag_value",
    "format_flag_value",
]

# What Darling needs on top of the user's flags: RbxTransport (QUIC) fails under
# Darling, the Pop-latency probe delays joins, and so on. They go into the
# Darling client's own file only, never into the user's flags or another
# client (Rosemary).
DARLING_FLAGS = {
    "FFlagUseRbxTransportClient": "False",
    "FFlagUseRbxTransportClient3": "False",
    "FFlagUseRbxTransportServer": "False",
    "FFlagShareRbxTransport": "False",
    "FFlagRbxTransportRuntime": "False",
    "DFFlagDebugDisableRbxTransportDummyClient": "True",
    "FFlagDebugDisableRbxTransportDummyClient": "True",
    "FStringRbxTransportDummyClientEnabledMinorVersions": "",
    "FStringRbxTransportDummyClientEnabledMinorVersions_PlaceFilter": "none",
    "DFIntRbxTransportDummyClientConnectionTimeoutMs": 0,
    "DFIntRbxTransportQuicHandshakeTimeoutMs": 0,
    # The Pop-latency STUN probe sends UDP bursts to 24 datacenters
    # every 0.5 s for ~5 s: it delays the first join (menu appears
    # late) and blocks the render switch after leaving a game.
    "DFFlagEnablePopLatencyProbe3": "False",
    "DFFlagAttachPopUdpProbeToGameJoin2": "False",
    # While on RakNet, Roblox opens a shadow RbxTransport connection
    # ("DummyClient will connect") that can never connect under
    # Darling; closing the session then waits for it, freezing the
    # return to the menu for seconds.
    "DFFlagRakNetFallbackToRbxTransportEvent": "False",
    "DFFlagRakNetFallbackToRbxTransportStatus": "False",
    "DFFlagConnectDummyServiceClientEarly": "False",
    "DFIntRbxTransportClientConnectionWaitIntervalMs": 0,
    # Disable client-side HTTP throttling and retry queues (which freeze
    # menu return for 5-6 s when batch thumbnails get throttled).
    "DFFlagHttpLocalThrottle": "False",
    "FFlagHttpLocalThrottle": "False",
    "DFIntHttpMaxRetries": 0,
    "DFIntHttpMaxRetryAfterSec": 0,
    "DFIntHttpRbxApiMaxThrottledQueueSize": 0,
    "DFIntHttpRetryAndLocalThrottleJitterMaxPercent": 0,
    "DFFlagHttpRetryOnExplicitHeader": "False",
    "DFFlagDebugSlimLoaderDisableHTTPRetry": "True",
    "FFlagDebugSlimLoaderDisableHTTPRetry": "True",
    "DFIntBatchThumbnailMaxWaitMs": 0,
    "DFIntBatchThumbnailMinWaitMs": 0,
    "DFIntBatchThumbnailExponentialInitialWaitMs": 0,
    "DFIntBatchThumbnailMaxExponentialRetries": 0,
    "DFIntBatchThumbnailAllowedExternalTimedOutRetries": 0,
    "DFIntLuaAppThumbnailsApiRetryTimeMultiplier": 0,
}

# The user's flags: the source of truth for the editor and for every client.
FAST_FLAGS_USER = locations.CONFIG_DIR / "fastflags.json"


def _read_flags(path):
    try:
        data = json.loads(path.read_text())
        return data if isinstance(data, dict) else {}
    except (OSError, ValueError):
        return {}


def load_bundle_flags():
    """ClientAppSettings.json of the Darling client (the user's flags plus DARLING_FLAGS)."""
    return _read_flags(locations.FAST_FLAGS)


def save_bundle_flags(flags):
    files._write_atomically(locations.FAST_FLAGS, json.dumps(flags, indent=2, ensure_ascii=False))


def load_fast_flags():
    """The user's Fast flags. Before the first save they are read from the
    Darling client's file, without the flags Quadra adds for Darling, so an
    existing setup carries over."""
    if FAST_FLAGS_USER.is_file():
        return _read_flags(FAST_FLAGS_USER)
    return {
        name: value
        for name, value in load_bundle_flags().items()
        if not (name in DARLING_FLAGS and DARLING_FLAGS[name] == value)
    }


def save_fast_flags(flags):
    """Keep the user's flags, and give the Darling client its copy (when that
    client is installed) with Darling's own flags on top."""
    files._write_atomically(FAST_FLAGS_USER, json.dumps(flags, indent=2, ensure_ascii=False))
    if locations.FAST_FLAGS.parent.is_dir():
        save_bundle_flags({**flags, **DARLING_FLAGS})


def parse_flag_value(text):
    """Turn what the user typed into the JSON value Roblox expects."""
    stripped = text.strip()
    lowered = stripped.lower()
    if lowered in ("true", "false"):
        return lowered == "true"
    try:
        return int(stripped)
    except ValueError:
        return stripped


def format_flag_value(value):
    if isinstance(value, bool):
        return "true" if value else "false"
    return str(value)
