"""Reading the client's logs, and keeping the logs folder small."""

import os
import re

from . import locations

__all__ = [
    "LOGS_MAX_BYTES",
    "_log_tail",
    "cleanup_logs",
    "exit_reason",
]


def _log_tail(log_path, limit=32768):
    try:
        with open(log_path, "rb") as file:
            file.seek(0, os.SEEK_END)
            file.seek(max(0, file.tell() - limit))
            return file.read(limit).decode(errors="replace")
    except (OSError, TypeError):
        return ""


def exit_reason(log_path):
    """A known cause for a game that quit, from its log, or None."""
    tail = _log_tail(log_path)
    if (
        "flow end: app_closed_for_update" in tail
        or "Found new version and the updater launched. Drain reporting and quit."
        in tail
    ):
        return "update_required"
    if "class WKWebView" in tail or "Selector setDetachesHiddenViews:" in tail:
        return "captcha"
    if (
        "X connection to " in tail
        and "broken (explicit kill or server shutdown)" in tail
    ):
        return "x11_broken"
    if (
        "[MangoHud]" in tail
        and "filesystem_error" in tail
        and re.search(r"Permission denied: [\"']/proc/\d+/fdinfo", tail)
    ):
        return "mangohud_fdinfo"
    return None


# Launch logs past this total are removed oldest first (count is `keep_logs`).
LOGS_MAX_BYTES = 200 * 1024 * 1024


def cleanup_logs(keep, current=None, max_bytes=LOGS_MAX_BYTES):
    """Remove old files written by the launcher itself: launch logs
    (launch-YYYYmmdd-HHMMSS.log) beyond `keep` or beyond `max_bytes` in
    total, and CiderDarling traces likewise. `current` is never removed;
    logs from run_debug.sh and other tools are left alone."""
    for glob, pattern in (
        ("launch-*.log", r"launch-\d{8}-\d{6}\.log"),
        ("ciderdarling-trace-*.jsonl", r"ciderdarling-trace-\d{8}-\d{6}\.jsonl"),
        ("metal-performance-*.jsonl", r"metal-performance-\d{8}-\d{6}-\d+\.jsonl"),
    ):
        logs = []
        for path in locations.LOGS.glob(glob):
            try:
                if (
                    re.fullmatch(pattern, path.name)
                    and path != current
                    and not path.is_symlink()
                ):
                    info = path.stat()
                    logs.append((info.st_mtime, info.st_size, path))
            except OSError:
                pass
        logs.sort(key=lambda entry: entry[0], reverse=True)
        total = 0
        for index, (_mtime, size, path) in enumerate(logs):
            total += size
            if index < max(0, keep) and total <= max_bytes:
                continue
            try:
                path.unlink()
            except OSError:
                pass
