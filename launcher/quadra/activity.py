"""Follows a Roblox player log and reports its events (see logevents).

Runs in a daemon thread; callbacks arrive on that thread, so GTK code must
hand off with GLib.idle_add. Handles the log being replaced or truncated and
skips absurdly long lines."""

import logging
import os
import threading

from . import logevents

log = logging.getLogger("quadra.activity")

POLL_SECONDS = 0.5
READ_SIZE = 64 * 1024
LINE_LIMIT = 256 * 1024


class ActivityWatcher:
    def __init__(self, log_path, callback):
        # A path, or a function returning the current path (None until the
        # log exists): the client of a new game writes a new file.
        self.log_path = log_path
        self.callback = callback
        self._stopped = threading.Event()
        self._thread = threading.Thread(
            target=self._run, name="quadra-activity", daemon=True
        )

    def start(self):
        self._thread.start()
        return self

    def stop(self):
        self._stopped.set()

    def _emit(self, raw):
        if len(raw) > LINE_LIMIT:
            return
        event = logevents.parse_line(raw.decode("utf-8", errors="replace"))
        if event is None:
            return
        try:
            self.callback(event)
        except Exception as error:  # a handler must not end the watcher
            log.warning("Activity handler failed: %s", error)

    def _run(self):
        identity = None
        position = 0
        fragment = b""
        while not self._stopped.is_set():
            try:
                path = self.log_path() if callable(self.log_path) else self.log_path
                if path is None:
                    raise FileNotFoundError
                with open(path, "rb") as handle:
                    status = os.fstat(handle.fileno())
                    now = (status.st_dev, status.st_ino)
                    if identity != now or status.st_size < position:
                        identity, position, fragment = now, 0, b""
                    handle.seek(position)
                    while not self._stopped.is_set():
                        chunk = handle.read(READ_SIZE)
                        if not chunk:
                            break
                        position += len(chunk)
                        pieces = (fragment + chunk).split(b"\n")
                        fragment = pieces.pop()
                        if len(fragment) > LINE_LIMIT:
                            fragment = b""
                        for raw in pieces:
                            self._emit(raw)
            except FileNotFoundError:
                pass
            except OSError as error:
                log.debug("Activity watcher: %s", error)
            self._stopped.wait(POLL_SECONDS)
