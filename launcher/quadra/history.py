"""Per-game server history, with a rejoin link for each server."""

import json
import os
import threading
import time
from pathlib import Path

MAX_GAMES = 50
MAX_SERVERS = 20
_lock = threading.Lock()


def _path(config_dir):
    return Path(config_dir) / "server-history.json"


def load(config_dir):
    try:
        data = json.loads(_path(config_dir).read_text())
    except (OSError, ValueError):
        return {}
    return data if isinstance(data, dict) else {}


def _save(config_dir, data):
    path = _path(config_dir)
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(".tmp")
    temporary.write_text(json.dumps(data, indent=1))
    os.replace(temporary, path)


def record(config_dir, place_id, job_id, *, name=None, server_ip=None, location=None, now=None):
    """Remember a server joined in a game. Newest first, bounded."""
    place_id = str(int(place_id))
    now = time.time() if now is None else now
    with _lock:
        data = load(config_dir)
        game = data.get(place_id) if isinstance(data.get(place_id), dict) else {}
        servers = [s for s in game.get("servers", []) if isinstance(s, dict) and s.get("job_id") != job_id]
        servers.insert(0, {"job_id": job_id, "server_ip": server_ip, "location": location, "joined_at": now})
        game = {"name": name or game.get("name") or "", "last_played": now, "servers": servers[:MAX_SERVERS]}
        data[place_id] = game
        for stale in sorted(data, key=lambda k: data[k].get("last_played", 0))[:max(0, len(data) - MAX_GAMES)]:
            del data[stale]
        _save(config_dir, data)


def set_location(config_dir, place_id, job_id, server_ip, location):
    """Attach a looked-up location to a recorded server."""
    place_id = str(int(place_id))
    with _lock:
        data = load(config_dir)
        for server in data.get(place_id, {}).get("servers", []):
            if server.get("job_id") == job_id:
                server["server_ip"] = server_ip
                server["location"] = location
                _save(config_dir, data)
                return


def recent_games(config_dir, limit=10):
    """[(place_id, entry)] most recently played first."""
    data = load(config_dir)
    games = [(k, v) for k, v in data.items() if isinstance(v, dict)]
    games.sort(key=lambda item: item[1].get("last_played", 0), reverse=True)
    return games[:limit]


def rejoin_uri(place_id, job_id=None):
    """roblox:// link that starts the game, in that server if it still exists."""
    uri = f"roblox://experiences/start?placeId={int(place_id)}"
    if job_id:
        uri += f"&gameInstanceId={job_id}"
    return uri


def forget(config_dir, place_id=None):
    with _lock:
        if place_id is None:
            data = {}
        else:
            data = load(config_dir)
            data.pop(str(place_id), None)
        _save(config_dir, data)
