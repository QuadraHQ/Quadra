"""Where the game server is: its address from the client's log, and its location."""

import json
import logging
import re
import threading
import time
import urllib.request
from pathlib import Path

from . import locations, logs

__all__ = [
    "SERVER_IP_PATTERN",
    "SERVER_LOCATION_RETRY",
    "SERVER_LOG_TAIL",
    "_latest_roblox_log",
    "_server_location_cache",
    "_server_location_lock",
    "detect_server_location",
    "extract_server_ip",
    "query_server_location",
]

SERVER_IP_PATTERN = re.compile(r"Connection accepted from\s+(\d{1,3}(?:\.\d{1,3}){3})")
# The connection line comes from the current game; its log grows for hours.
SERVER_LOG_TAIL = 256 * 1024
# Failed lookups are not repeated for this long (successes are kept).
SERVER_LOCATION_RETRY = 300

_server_location_cache = {}
_server_location_lock = threading.Lock()


def _latest_roblox_log():
    """Return the newest Roblox Player log file inside the selected Darling
    prefix, or in the launcher's logs folder."""
    candidates_dirs = [
        locations.DARLING_HOME / "Library" / "Logs" / "Roblox",
        locations.LOGS,
    ]
    for log_dir in candidates_dirs:
        newest, newest_time = None, None
        try:
            for path in log_dir.glob("*_Player_*.log"):
                if "CrashHandler" in path.name:
                    continue
                try:
                    modified = path.stat().st_mtime
                except OSError:
                    continue
                if newest_time is None or modified > newest_time:
                    newest, newest_time = path, modified
        except OSError:
            continue
        if newest:
            return newest
    return None


def extract_server_ip(log_path=None):
    """Read the end of the latest Roblox log and extract the connected server IP."""
    if log_path is None:
        log_path = _latest_roblox_log()
    if not log_path or not Path(log_path).is_file():
        return None
    matches = SERVER_IP_PATTERN.findall(logs._log_tail(log_path, SERVER_LOG_TAIL))
    return matches[-1] if matches else None


def query_server_location(ip_address):
    """Query the RoValra API for the geolocation of a server IP. Blocks for
    up to a few seconds: call it off the GTK thread."""
    import ipaddress

    try:
        address = ipaddress.IPv4Address(ip_address)
    except ValueError:
        return None
    if not address.is_global:
        return None  # nothing to look up, and nothing to send to anyone
    with _server_location_lock:
        cached = _server_location_cache.get(str(address))
    if cached and (
        cached[0] is not None or time.monotonic() - cached[1] < SERVER_LOCATION_RETRY
    ):
        return cached[0]
    url = f"https://apis.rovalra.com/v1/geolocation?ip={address}"
    result = None
    try:
        request = urllib.request.Request(url, headers={"User-Agent": "Quadra"})
        with urllib.request.urlopen(request, timeout=5) as response:
            data = json.loads(response.read(65536).decode())
        location = data.get("location", {}) if isinstance(data, dict) else {}
        location = location if isinstance(location, dict) else {}
        parts = [location.get("city"), location.get("region"), location.get("country")]
        parts = [str(part)[:64] for part in parts if part]
        result = ", ".join(parts) if parts else None
    except Exception as e:
        logging.getLogger("quadra").warning("RoValra query failed: %s", e)
    with _server_location_lock:
        _server_location_cache[str(address)] = (result, time.monotonic())
    return result


def detect_server_location():
    """Extract the current server IP and query its geolocation (blocking)."""
    ip = extract_server_ip()
    if not ip:
        return None, None
    return ip, query_server_location(ip)
