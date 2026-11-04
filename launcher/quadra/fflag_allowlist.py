"""Roblox's public fast flag allowlist.

Roblox publishes the flags a client honours as the application setting
DFStringAllowedPublicFlags (a base64 JSON object with an "Allowed" list).
This fetches and caches it, so the launcher can warn about flags that will be
ignored instead of keeping a list that goes stale. The idea comes from
Voidstrap's RobloxFastFlagAllowlist (MIT).

Offline, or if Roblox changes the format, the last cached list is used; with
no list at all every flag counts as allowed (nothing is hidden or blocked on
a guess)."""

import base64
import json
import logging
import os
import threading
import time
import urllib.request
from pathlib import Path

log = logging.getLogger("quadra.fflags")

SETTINGS_URL = (
    "https://clientsettingscdn.roblox.com/v2/settings/application/PCDesktopClient"
)
SETTING_NAME = "DFStringAllowedPublicFlags"
MAX_BYTES = 4 * 1024 * 1024
REFRESH_SECONDS = 6 * 3600

_lock = threading.Lock()
_flags = None  # frozenset, or None until loaded
_loaded_at = 0.0


def parse(payload):
    """The allowed flag names in a client settings response, or None."""
    try:
        data = json.loads(payload)
        encoded = data["applicationSettings"][SETTING_NAME]
        allowed = json.loads(base64.b64decode(encoded))["Allowed"]
    except (ValueError, KeyError, TypeError):
        return None
    if not isinstance(allowed, list):
        return None
    names = frozenset(name for name in allowed if isinstance(name, str) and name)
    return names or None


def _cache_path(cache_dir):
    return Path(cache_dir) / "fflag-allowlist.json"


def load_cached(cache_dir):
    global _flags
    try:
        names = json.loads(_cache_path(cache_dir).read_text()).get("allowed")
    except (OSError, ValueError, AttributeError):
        return None
    if not isinstance(names, list):
        return None
    with _lock:
        _flags = frozenset(n for n in names if isinstance(n, str))
        return _flags


def refresh(cache_dir, *, force=False, opener=urllib.request.urlopen):
    """Fetch the list (blocking: call off the GTK thread). Returns the set in
    use afterwards, which may be the cached one if the fetch failed."""
    global _flags, _loaded_at
    with _lock:
        fresh = _flags is not None and time.monotonic() - _loaded_at < REFRESH_SECONDS
    if fresh and not force:
        return _flags
    if _flags is None:
        load_cached(cache_dir)
    try:
        request = urllib.request.Request(SETTINGS_URL, headers={"User-Agent": "Quadra"})
        with opener(request, timeout=10) as response:
            names = parse(response.read(MAX_BYTES))
    except Exception as error:
        log.info("Using the last known flag allowlist: %s", error)
        names = None
    if names:
        with _lock:
            _flags = names
            _loaded_at = time.monotonic()
        try:
            path = _cache_path(cache_dir)
            path.parent.mkdir(parents=True, exist_ok=True)
            temporary = path.with_suffix(".tmp")
            temporary.write_text(json.dumps({"allowed": sorted(names)}))
            os.replace(temporary, path)
        except OSError as error:
            log.debug("Could not cache the flag allowlist: %s", error)
    return _flags


def is_allowed(name):
    """True if Roblox honours the flag (or if no list is known)."""
    with _lock:
        return not _flags or name in _flags


def unknown(flags):
    """The names in `flags` that Roblox ignores, sorted."""
    return sorted(name for name in flags if not is_allowed(name))


def known():
    with _lock:
        return _flags
