import contextlib
import io
import json
import os
from pathlib import Path
import shutil
import socket
import struct
import tempfile
import threading
import time
import unittest
from unittest.mock import patch

from quadra import discord


class FakeDiscord:
    """A Discord IPC socket answering like arRPC; records each connection's
    activities (None for a clear) and whether it was closed."""

    def __init__(self, directory):
        self.connections = []
        self.lock = threading.Condition()
        self.server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.server.bind(str(Path(directory) / "discord-ipc-0"))
        self.server.listen(4)
        threading.Thread(target=self._accept, daemon=True).start()

    def _accept(self):
        while True:
            try:
                client, _ = self.server.accept()
            except OSError:
                return
            record = {"activities": [], "closed": False}
            with self.lock:
                self.connections.append(record)
            threading.Thread(target=self._serve, args=(client, record), daemon=True).start()

    def _serve(self, client, record):
        def read(length):
            data = b""
            while len(data) < length:
                chunk = client.recv(length - len(data))
                if not chunk:
                    raise EOFError
                data += chunk
            return data

        def send(message):
            body = json.dumps(message).encode()
            client.sendall(struct.pack("<II", 1, len(body)) + body)

        try:
            while True:
                opcode, length = struct.unpack("<II", read(8))
                message = json.loads(read(length))
                if opcode == 0:
                    send({"cmd": "DISPATCH", "evt": "READY", "data": {"v": 1}, "nonce": None})
                    continue
                activity = message["args"].get("activity")
                send({"cmd": "SET_ACTIVITY", "data": activity, "evt": None, "nonce": message["nonce"]})
                with self.lock:
                    record["activities"].append(activity)
                    self.lock.notify_all()
        except (EOFError, OSError):
            pass
        finally:
            client.close()
            with self.lock:
                record["closed"] = True
                self.lock.notify_all()

    def wait(self, predicate, timeout=5):
        with self.lock:
            if not self.lock.wait_for(lambda: predicate(self.connections), timeout):
                raise AssertionError(f"timed out; connections: {self.connections}")

    def close(self):
        self.server.close()


class DiscordPresenceTests(unittest.TestCase):
    def setUp(self):
        # Unix socket paths are short; avoid long temporary directories.
        directory = tempfile.mkdtemp(prefix="quadra-rpc-", dir="/tmp" if os.path.isdir("/tmp") else None)
        self.addCleanup(shutil.rmtree, directory, True)
        environment = patch.dict(os.environ, {"XDG_RUNTIME_DIR": directory, "TMPDIR": directory})
        environment.start()
        self.addCleanup(environment.stop)
        self.discord = FakeDiscord(directory)
        self.addCleanup(self.discord.close)
        pauses = patch.object(discord.DiscordRPC, "_CLEAR_AGAIN", (0.05, 0.05))
        pauses.start()
        self.addCleanup(pauses.stop)

    def start(self, **presence):
        rpc = discord.DiscordRPC()
        self.addCleanup(rpc._worker.join, 15)
        self.addCleanup(rpc.close)
        rpc.queue_presence(**presence)
        return rpc

    def test_clears_again_on_the_same_connection(self):
        rpc = self.start(details="In Main Menu")
        self.discord.wait(lambda c: c and c[0]["activities"])
        rpc.close()
        self.discord.wait(lambda c: c[0]["closed"])
        connection, = self.discord.connections
        self.assertEqual(connection["activities"][1:], [None, None, None])
        self.assertEqual(connection["activities"][0]["assets"]["large_image"], discord.LOGO)

    def test_newer_presence_stops_the_repeated_clears(self):
        with patch.object(discord.DiscordRPC, "_CLEAR_AGAIN", (0.3, 30.0)):
            old = self.start(details="Playing Roblox")
            self.discord.wait(lambda c: c and c[0]["activities"])
            old.close()
            self.discord.wait(lambda c: c[0]["activities"][-1] is None)
            self.start(details="In Main Menu")
            started = time.monotonic()
            self.discord.wait(lambda c: c[0]["closed"])
        self.assertLess(time.monotonic() - started, 5)
        self.assertEqual(self.discord.connections[0]["activities"][1:], [None])

    def test_finds_sockets_shared_by_sandboxed_discord_clients(self):
        runtime = os.environ["XDG_RUNTIME_DIR"]
        shared = [
            Path(runtime, "app", "com.discordapp.Discord", "discord-ipc-0"),
            Path(runtime, ".flatpak", "dev.vencord.Vesktop", "xdg-run", "discord-ipc-0"),
            Path(runtime, "snap.discord", "discord-ipc-1"),
        ]
        for path in shared:
            path.parent.mkdir(parents=True)
            path.touch()
        rpc = discord.DiscordRPC()
        self.addCleanup(rpc._worker.join, 15)
        self.addCleanup(rpc.close)
        found = rpc._find_socket()
        self.assertEqual(found[0], str(Path(runtime, "discord-ipc-0")))
        self.assertCountEqual(found[1:], [str(path) for path in shared])

    def test_activity_branding_and_buttons(self):
        activity = discord.DiscordRPC._activity(
            details="Game", buttons=[("Game page", "https://www.roblox.com/games/1"),
                                     ("Get Quadra", discord.WEBSITE_URL), ("Third", "https://example.com")])
        self.assertEqual(activity["assets"]["large_image"], discord.LOGO)
        self.assertEqual(activity["buttons"], [{"label": "Game page", "url": "https://www.roblox.com/games/1"},
                                               {"label": "Get Quadra", "url": discord.WEBSITE_URL}])
        self.assertNotIn("buttons", discord.DiscordRPC._activity(details="Menu"))


class FakeRobloxApi:
    def __init__(self):
        self.name = "Natural Disaster Survival"
        self.creator = "Stickmasterluke"
        self.icon_state = "Completed"
        self.icon = "https://tr.rbxcdn.com/icon-1.png"
        self.requests = []

    def __call__(self, url):
        self.requests.append(url)
        if "/universe" in url:
            return {"universeId": 7}
        if "games.roblox.com" in url:
            return {"data": [{"name": self.name, "creator": {"name": self.creator}}]}
        return {"data": [{"state": self.icon_state, "imageUrl": self.icon}]}


class GameLookupTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.mkdtemp(prefix="quadra-games-")
        self.addCleanup(shutil.rmtree, directory, True)
        for change in (
            patch.object(discord, "CACHE_FILE", Path(directory) / "game_cache.json"),
            patch.dict(discord._GAME_CACHE, clear=True),
        ):
            change.start()
            self.addCleanup(change.stop)
        self.api = FakeRobloxApi()
        api = patch.object(discord, "_public_json", self.api)
        api.start()
        self.addCleanup(api.stop)

    def test_a_placeholder_name_is_not_shown(self):
        self.api.name = "[TITLE UNAVAILABLE]"
        self.assertIsNone(discord.fetch_game_info(1))
        self.assertNotIn(1, discord._GAME_CACHE)

    def test_a_refresh_keeps_what_the_new_answer_lacks(self):
        first = discord.fetch_game_info(1)
        self.api.name = "[TITLE UNAVAILABLE]"
        self.api.creator = "[UNKNOWN]"
        self.api.icon_state = "Pending"
        self.api.icon = "https://tr.rbxcdn.com/placeholder.png"
        self.assertEqual(discord.fetch_game_info(1, refresh=True), first)

    def test_a_refresh_takes_newer_details_and_saves_them(self):
        discord.fetch_game_info(1)
        self.api.name = "Natural Disaster Survival [UPDATE]"
        refreshed = discord.fetch_game_info(1, refresh=True)
        self.assertEqual(refreshed["name"], "Natural Disaster Survival [UPDATE]")
        saved = json.loads(discord.CACHE_FILE.read_text())
        self.assertEqual(saved["1"]["name"], "Natural Disaster Survival [UPDATE]")
        self.assertEqual(sum("/universe" in url for url in self.api.requests), 1)

    def test_an_icon_still_being_made_is_not_used(self):
        self.api.icon_state = "Pending"
        self.assertIsNone(discord.fetch_game_info(1)["icon_url"])

    def test_cached_placeholder_names_are_dropped(self):
        entry = {"universe_id": 7, "name": "[TITLE UNAVAILABLE]", "creator": "", "icon_url": None}
        self.assertIsNone(discord._cache_entry(1, entry))

    def test_the_game_being_played_is_looked_up_again(self):
        directory = tempfile.mkdtemp(prefix="quadra-log-")
        self.addCleanup(shutil.rmtree, directory, True)
        log_path = Path(directory) / "roblox.log"
        log_path.write_text("[FLog::Output] ! Joining game 'abc' place 1 at 10.0.0.1\n")
        seen = []
        changed = threading.Condition()

        def on_change(info, _generation):
            with changed:
                seen.append(info)
                changed.notify_all()

        with patch.object(discord.GameActivityTracker, "_LOOKUP_SECONDS", 0.05):
            tracker = discord.GameActivityTracker(str(log_path), on_change)
            self.addCleanup(tracker.stop)
            with changed:
                self.assertTrue(changed.wait_for(lambda: any(i and i.get("creator") for i in seen), 5))
                self.api.name = "Natural Disaster Survival [UPDATE]"
                self.assertTrue(changed.wait_for(lambda: seen[-1]["name"].endswith("[UPDATE]"), 5))
        self.assertTrue(seen[0]["loading"])


class ActivityTextTests(unittest.TestCase):
    def test_text_fits_discord_limits(self):
        activity = discord.DiscordRPC._activity(details="x" * 200, state="A", large_text="Q")
        self.assertEqual(len(activity["details"]), 128)
        self.assertTrue(activity["details"].endswith("…"))
        self.assertEqual(activity["state"], "A ")
        self.assertEqual(activity["assets"]["large_text"], "Q ")
        self.assertEqual(activity["type"], 0)


class DiscordProblemTests(unittest.TestCase):
    def test_a_missing_discord_is_reported_once(self):
        directory = tempfile.mkdtemp(prefix="quadra-rpc-", dir="/tmp")
        self.addCleanup(shutil.rmtree, directory, True)
        empty = {"XDG_RUNTIME_DIR": directory, "TMPDIR": directory, "TMP": directory, "TEMP": directory}
        output = io.StringIO()
        with patch.dict(os.environ, empty), patch.object(
            discord.DiscordRPC, "_find_socket", return_value=[]
        ), contextlib.redirect_stdout(output):
            rpc = discord.DiscordRPC()
            self.addCleanup(rpc.close)
            self.assertFalse(rpc.connect())
            self.assertFalse(rpc.connect())
        self.assertEqual(output.getvalue(), "Discord presence: no Discord app found (no discord-ipc socket)\n")

    def test_discord_reasons_are_read(self):
        self.assertEqual(discord.DiscordRPC._reason({"data": {"message": "Invalid asset"}}), "Invalid asset")
        self.assertEqual(discord.DiscordRPC._reason(b'{"code": 4000, "message": "Invalid Client ID"}'), "Invalid Client ID")
        self.assertEqual(discord.DiscordRPC._reason(b""), "no reason given")


if __name__ == "__main__":
    unittest.main()
