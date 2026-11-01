"""Public Roblox metadata for the launcher: game icons and last-update times."""

import json
import logging
import os
import urllib.parse
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

log = logging.getLogger("quadra.gamemeta")

ICON_LIMIT = 2 * 1024 * 1024
RECENT_UPDATE_HOURS = 24


def _safe_icon_url(url):
    parts = urllib.parse.urlsplit(url or "")
    return parts.scheme == "https" and bool(parts.hostname) and parts.hostname.endswith(".rbxcdn.com")


def icon_path(cache_dir, place_id):
    return Path(cache_dir) / "game-icons" / f"{int(place_id)}.png"


def fetch_icon(cache_dir, place_id, url, *, opener=urllib.request.urlopen):
    """Download a game's icon once; returns its path or None."""
    path = icon_path(cache_dir, place_id)
    if path.is_file():
        return path
    if not _safe_icon_url(url):
        return None
    try:
        with opener(urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"}), timeout=8) as response:
            body = response.read(ICON_LIMIT + 1)
        if not body or len(body) > ICON_LIMIT:
            return None
        path.parent.mkdir(parents=True, exist_ok=True)
        temporary = path.with_suffix(".tmp")
        temporary.write_bytes(body)
        os.replace(temporary, path)
        return path
    except Exception as error:
        log.debug("Game icon unavailable: %s", error)
        return None


def parse_time(text):
    """Seconds since the epoch for an ISO 8601 time from Roblox, or None."""
    if not isinstance(text, str):
        return None
    try:
        parsed = datetime.fromisoformat(text.replace("Z", "+00:00"))
    except ValueError:
        return None
    if parsed.tzinfo is None:
        parsed = parsed.replace(tzinfo=timezone.utc)
    return parsed.timestamp()


def last_updated(universe_id, *, opener=urllib.request.urlopen):
    """When the game was last updated (epoch seconds), or None."""
    url = f"https://games.roblox.com/v1/games?universeIds={int(universe_id)}"
    try:
        with opener(urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"}), timeout=6) as response:
            data = json.loads(response.read(1024 * 1024))
        return parse_time(data["data"][0]["updated"])
    except Exception as error:
        log.debug("Game update time unavailable: %s", error)
        return None


def recently_updated(updated, now, hours=RECENT_UPDATE_HOURS):
    """Hours since `updated` if that is under `hours`, else None."""
    if updated is None or updated > now + 60:
        return None
    age = (now - updated) / 3600
    return age if age < hours else None
