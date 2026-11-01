"""Roblox player log events, from one table.

Substrings follow the markers Bloxstrap, AppleBlox and Voidstrap match. They
are plain `in` checks (cheap on every line) with an optional regex to pull
the data out. Whether the current client still writes each one is not
guaranteed: a missing marker only means that event is never reported."""

import re
from dataclasses import dataclass

# Event names
JOINING = "joining"            # data: job_id, place_id, server_ip
JOINED = "joined"              # data: server_ip, port
UDMUX = "udmux"                # data: udmux_ip, server_ip
DISCONNECTED = "disconnected"
LEAVING = "leaving"
TELEPORT = "teleport"
RETURN_TO_APP = "return_to_app"
CRASH = "crash"                # data: text
SERVER_PREFIX = "server_prefix"


@dataclass(frozen=True)
class Event:
    name: str
    data: dict


_ENTRIES = (
    (JOINING, "! Joining game",
     re.compile(r"! Joining game '([0-9a-fA-F-]{36})' place (\d+) at ([0-9.]+)"),
     ("job_id", "place_id", "server_ip")),
    (UDMUX, "UDMUX Address = ",
     re.compile(r"UDMUX Address = ([0-9.]+), Port = \d+ \| RCC Server Address = ([0-9.]+)"),
     ("udmux_ip", "server_ip")),
    (JOINED, "[FLog::Network] serverId:",
     re.compile(r"serverId: ([0-9.]+)\|(\d+)"),
     ("server_ip", "port")),
    (DISCONNECTED, "[FLog::Network] Time to disconnect replication data:", None, ()),
    (LEAVING, "[FLog::SingleSurfaceApp] leaveUGCGameInternal", None, ()),
    (TELEPORT, "[FLog::SingleSurfaceApp] initiateTeleport", None, ()),
    (RETURN_TO_APP, "[FLog::SingleSurfaceApp] returnToLuaApp", None, ()),
    (CRASH, "[FLog::CrashReportLog]",
     re.compile(r"\[FLog::CrashReportLog\] (.*)"), ("text",)),
    (SERVER_PREFIX, "[FLog::Output] Server Prefix: ",
     re.compile(r"Server Prefix: (.*)"), ("text",)),
)


def parse_line(line):
    """The Event a log line reports, or None."""
    for name, marker, pattern, fields in _ENTRIES:
        if marker not in line:
            continue
        data = {}
        if pattern is not None:
            match = pattern.search(line)
            if not match:
                continue
            data = dict(zip(fields, match.groups()))
        return Event(name, data)
    return None
