"""Mod crash guard.

The custom mods folder (`modifications/`) is applied on top of Roblox's
content. If a session that started with new mods crashes soon after, those
mods are the likely cause, so they are set aside (renamed `<name>.disabled`,
which is reversible) and the user is told. The unit of quarantine is a
top-level entry of the folder. The approach follows Voidstrap's ModCrashGuard.

State: `last_good` is the set of units present in the last session that ended
cleanly or ran past SETTLE_SECONDS; None means no baseline yet, in which case
nothing is quarantined (there is no way to tell which unit is new)."""

import json
import os
import time
from pathlib import Path

SETTLE_SECONDS = 300
IGNORED = {"README.txt", ".DS_Store"}
SUFFIX = ".disabled"


def _state_path(config_dir):
    return Path(config_dir) / "mod-guard.json"


def _load(config_dir):
    try:
        data = json.loads(_state_path(config_dir).read_text())
    except (OSError, ValueError):
        data = {}
    return data if isinstance(data, dict) else {}


def _save(config_dir, data):
    path = _state_path(config_dir)
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(".tmp")
    temporary.write_text(json.dumps(data, indent=1))
    os.replace(temporary, path)


def units(mods_dir):
    """Active top-level entries of the mods folder."""
    try:
        names = os.listdir(mods_dir)
    except OSError:
        return []
    return sorted(n for n in names if n not in IGNORED and not n.endswith(SUFFIX))


def disabled_units(mods_dir):
    try:
        names = os.listdir(mods_dir)
    except OSError:
        return []
    return sorted(n[:-len(SUFFIX)] for n in names if n.endswith(SUFFIX))


def is_disabled(name):
    return name.endswith(SUFFIX)


def begin_session(config_dir, mods_dir, *, now=None):
    data = _load(config_dir)
    data["session"] = {"units": units(mods_dir), "started": time.time() if now is None else now}
    _save(config_dir, data)


def end_session(config_dir, mods_dir, *, crashed, now=None):
    """Settle the session begun by begin_session. Returns the units that were
    set aside (empty when nothing was)."""
    data = _load(config_dir)
    session = data.pop("session", None)
    if not isinstance(session, dict):
        return []
    now = time.time() if now is None else now
    ran = now - session.get("started", now)
    used = [u for u in session.get("units", []) if isinstance(u, str)]
    last_good = data.get("last_good")
    quarantined = []
    if crashed and ran < SETTLE_SECONDS and isinstance(last_good, list):
        for unit in used:
            if unit in last_good:
                continue
            source = Path(mods_dir) / unit
            target = Path(mods_dir) / (unit + SUFFIX)
            if source.exists() and not target.exists():
                try:
                    source.rename(target)
                except OSError:
                    continue
                quarantined.append(unit)
    elif not crashed or ran >= SETTLE_SECONDS:
        data["last_good"] = used
    if quarantined:
        data["last_quarantined"] = {"units": quarantined, "at": now}
    _save(config_dir, data)
    return quarantined


def last_quarantined(config_dir):
    entry = _load(config_dir).get("last_quarantined")
    return entry if isinstance(entry, dict) else None


def clear_notice(config_dir):
    data = _load(config_dir)
    if data.pop("last_quarantined", None) is not None:
        _save(config_dir, data)


def restore(mods_dir, name):
    """Re-enable a set-aside unit."""
    source = Path(mods_dir) / (name + SUFFIX)
    target = Path(mods_dir) / name
    if source.exists() and not target.exists():
        source.rename(target)
        return True
    return False
