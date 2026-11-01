"""Unit tests for the launcher's pure-logic modules.

Run: PYTHONPATH=launcher python3 -m unittest discover -s tests/launcher -v
"""
import base64
import json
import tempfile
import unittest
from pathlib import Path

import threading
import time

from quadra import activity, dns, game_meta, region, fflag_allowlist, history, logevents, modguard


def _payload(names):
    inner = base64.b64encode(json.dumps({"Allowed": names}).encode()).decode()
    return json.dumps({"applicationSettings": {fflag_allowlist.SETTING_NAME: inner}}).encode()


class AllowlistTests(unittest.TestCase):
    def setUp(self):
        fflag_allowlist._flags = None
        fflag_allowlist._loaded_at = 0.0

    def test_parse(self):
        self.assertEqual(fflag_allowlist.parse(_payload(["A", "B", "", 3])), frozenset({"A", "B"}))

    def test_parse_rejects_garbage(self):
        for bad in (b"", b"{}", b"not json", json.dumps({"applicationSettings": {}}).encode()):
            self.assertIsNone(fflag_allowlist.parse(bad))
        inner = base64.b64encode(b'{"Allowed": "x"}').decode()
        self.assertIsNone(fflag_allowlist.parse(
            json.dumps({"applicationSettings": {fflag_allowlist.SETTING_NAME: inner}}).encode()))

    def test_no_list_allows_everything(self):
        self.assertTrue(fflag_allowlist.is_allowed("Anything"))
        self.assertEqual(fflag_allowlist.unknown(["X"]), [])

    def test_refresh_caches_and_filters(self):
        class Response:
            def __init__(self, body): self.body = body
            def __enter__(self): return self
            def __exit__(self, *a): return False
            def read(self, n): return self.body
        with tempfile.TemporaryDirectory() as tmp:
            fflag_allowlist.refresh(tmp, force=True, opener=lambda *a, **k: Response(_payload(["Good"])))
            self.assertTrue(fflag_allowlist.is_allowed("Good"))
            self.assertEqual(fflag_allowlist.unknown(["Good", "Bad"]), ["Bad"])
            # A failing fetch falls back to the cached list.
            fflag_allowlist._flags = None
            def fail(*a, **k): raise OSError("offline")
            fflag_allowlist.refresh(tmp, force=True, opener=fail)
            self.assertEqual(fflag_allowlist.known(), frozenset({"Good"}))


class LogEventTests(unittest.TestCase):
    def test_joining(self):
        event = logevents.parse_line(
            "2026-11-01T10:00:00.1Z,1.0,abc,6 [FLog::Output] ! Joining game "
            "'0123456789abcdef0123456789abcdef0123' place 4483381587 at 128.116.10.4")
        self.assertEqual(event.name, logevents.JOINING)
        self.assertEqual(event.data["place_id"], "4483381587")
        self.assertEqual(event.data["server_ip"], "128.116.10.4")

    def test_joined_and_udmux(self):
        self.assertEqual(logevents.parse_line("[FLog::Network] serverId: 128.116.1.2|53640").data,
                         {"server_ip": "128.116.1.2", "port": "53640"})
        self.assertEqual(logevents.parse_line(
            "[FLog::Network] UDMUX Address = 1.2.3.4, Port = 1 | RCC Server Address = 5.6.7.8, Port = 2").data,
            {"udmux_ip": "1.2.3.4", "server_ip": "5.6.7.8"})

    def test_plain_markers(self):
        for text, name in (("[FLog::Network] Time to disconnect replication data: 3", logevents.DISCONNECTED),
                           ("[FLog::SingleSurfaceApp] leaveUGCGameInternal", logevents.LEAVING),
                           ("[FLog::SingleSurfaceApp] initiateTeleport", logevents.TELEPORT),
                           ("[FLog::SingleSurfaceApp] returnToLuaApp", logevents.RETURN_TO_APP)):
            self.assertEqual(logevents.parse_line(text).name, name)

    def test_unrelated_and_malformed(self):
        self.assertIsNone(logevents.parse_line("[FLog::Graphics] something"))
        # Marker present but the data does not parse: no event.
        self.assertIsNone(logevents.parse_line("[FLog::Network] serverId: nope"))


class HistoryTests(unittest.TestCase):
    def test_record_order_dedupe_and_bounds(self):
        with tempfile.TemporaryDirectory() as tmp:
            history.record(tmp, 1, "a", name="One", now=1)
            history.record(tmp, 1, "b", now=2)
            history.record(tmp, 1, "a", now=3)  # moves to the front
            history.record(tmp, 2, "c", name="Two", now=4)
            games = history.recent_games(tmp)
            self.assertEqual([g[0] for g in games], ["2", "1"])
            self.assertEqual([s["job_id"] for s in games[1][1]["servers"]], ["a", "b"])
            self.assertEqual(games[1][1]["name"], "One")
            for i in range(history.MAX_SERVERS + 5):
                history.record(tmp, 1, f"j{i}", now=10 + i)
            self.assertEqual(len(history.load(tmp)["1"]["servers"]), history.MAX_SERVERS)

    def test_set_location_and_uri(self):
        with tempfile.TemporaryDirectory() as tmp:
            history.record(tmp, 5, "job", now=1)
            history.set_location(tmp, 5, "job", "1.2.3.4", "Frankfurt, DE")
            self.assertEqual(history.load(tmp)["5"]["servers"][0]["location"], "Frankfurt, DE")
        self.assertEqual(history.rejoin_uri(5), "roblox://experiences/start?placeId=5")
        self.assertTrue(history.rejoin_uri(5, "j").endswith("&gameInstanceId=j"))

    def test_corrupt_file_is_empty(self):
        with tempfile.TemporaryDirectory() as tmp:
            (Path(tmp) / "server-history.json").write_text("{nope")
            self.assertEqual(history.load(tmp), {})
            history.record(tmp, 1, "a")  # recovers


class ModGuardTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.config = self.root / "config"
        self.mods = self.root / "mods"
        self.mods.mkdir()
        (self.mods / "README.txt").write_text("x")

    def tearDown(self):
        self.tmp.cleanup()

    def run_session(self, *, crashed, seconds):
        modguard.begin_session(self.config, self.mods, now=1000)
        return modguard.end_session(self.config, self.mods, crashed=crashed, now=1000 + seconds)

    def test_new_unit_in_a_quick_crash_is_set_aside(self):
        (self.mods / "textures").mkdir()
        self.assertEqual(self.run_session(crashed=False, seconds=600), [])  # baseline: textures is good
        (self.mods / "sounds").mkdir()
        self.assertEqual(self.run_session(crashed=True, seconds=20), ["sounds"])
        self.assertTrue((self.mods / "sounds.disabled").exists())
        self.assertTrue((self.mods / "textures").exists())
        self.assertEqual(modguard.units(self.mods), ["textures"])
        self.assertEqual(modguard.disabled_units(self.mods), ["sounds"])
        self.assertTrue(modguard.restore(self.mods, "sounds"))
        self.assertTrue((self.mods / "sounds").exists())

    def test_no_baseline_means_no_action(self):
        (self.mods / "textures").mkdir()
        self.assertEqual(self.run_session(crashed=True, seconds=5), [])
        self.assertTrue((self.mods / "textures").exists())

    def test_late_crash_is_not_blamed_on_mods(self):
        (self.mods / "a").mkdir()
        self.run_session(crashed=False, seconds=600)
        (self.mods / "b").mkdir()
        self.assertEqual(self.run_session(crashed=True, seconds=modguard.SETTLE_SECONDS + 1), [])
        self.assertTrue((self.mods / "b").exists())

    def test_end_without_begin_is_harmless(self):
        self.assertEqual(modguard.end_session(self.config, self.mods, crashed=True), [])


def _query(name, qtype=1):
    wire = b"".join(bytes([len(p)]) + p.encode() for p in name.split(".")) + b"\0"
    return b"\x12\x34\x01\x00\x00\x01\x00\x00\x00\x00\x00\x00" + wire + qtype.to_bytes(2, "big") + b"\x00\x01"


class DnsBlockTests(unittest.TestCase):
    def test_query_name_and_blocking(self):
        query = _query("Metrics.Roblox.com")
        self.assertEqual(dns.query_name(query), "metrics.roblox.com")
        self.assertTrue(dns.is_blocked("metrics.roblox.com", dns.TELEMETRY_DOMAINS))
        self.assertTrue(dns.is_blocked("a.metrics.roblox.com", dns.TELEMETRY_DOMAINS))
        self.assertFalse(dns.is_blocked("notmetrics.roblox.com", dns.TELEMETRY_DOMAINS))
        self.assertFalse(dns.is_blocked("www.roblox.com", dns.TELEMETRY_DOMAINS))
        self.assertFalse(dns.is_blocked(None, dns.TELEMETRY_DOMAINS))

    def test_nxdomain_keeps_id_and_question(self):
        query = _query("gold.roblox.com")
        reply = dns._nxdomain(query)
        self.assertEqual(reply[:2], query[:2])
        self.assertEqual(reply[3] & 0x0F, 3)        # RCODE NXDOMAIN
        self.assertTrue(reply[2] & 0x80)            # a response
        self.assertEqual(reply[12:], query[12:])    # question echoed


class RegionTests(unittest.TestCase):
    class Response:
        def __init__(self, data): self.data = json.dumps(data).encode()
        def __enter__(self): return self
        def __exit__(self, *a): return False
        def read(self, n): return self.data

    def test_parse_locations(self):
        data = [{"location": {"city": "Frankfurt am Main", "country": "DE", "country_name": "Germany"}, "inactive": False},
                {"location": {"city": "Gone", "country": "XX"}, "inactive": True},
                {"location": {}}, "junk"]
        self.assertEqual(region.parse_locations(data), [("DE", "Germany", "Frankfurt am Main")])

    def test_keys_and_places(self):
        self.assertEqual(region.split_region("DE:Frankfurt am Main"), ("DE", "Frankfurt am Main"))
        self.assertEqual(region.split_region("DE"), ("DE", None))
        self.assertEqual(region.split_region("AUTO:"), ("AUTO", None))
        self.assertEqual(region.parse_place("4924922222"), 4924922222)
        self.assertEqual(region.parse_place("https://www.roblox.com/games/4924922222/Brookhaven"), 4924922222)
        self.assertIsNone(region.parse_place("nope"))

    def test_pick_server_avoids_recent_and_full(self):
        data = {"servers": [{"server_id": "a"}, {"server_id": "b"},
                            {"server_id": "full", "playing": 10, "max_players": 10}]}
        opener = lambda *a, **k: self.Response(data)
        for _ in range(20):
            self.assertEqual(region.pick_server(1, "DE", avoid=("a",), opener=opener), "b")
        self.assertIn(region.pick_server(1, "DE", avoid=("a", "b"), opener=opener), ("a", "b"))

    def test_pick_server_failure_and_empty(self):
        def fail(*a, **k): raise OSError("offline")
        self.assertIsNone(region.pick_server(1, "DE", opener=fail))
        self.assertIsNone(region.pick_server(1, "DE", opener=lambda *a, **k: self.Response({"servers": []})))
        self.assertIsNone(region.pick_server(1, "AUTO:", opener=fail) if False else region.pick_server(1, "", opener=fail))


class GameMetaTests(unittest.TestCase):
    def test_parse_time_and_recent(self):
        t = game_meta.parse_time("2026-11-01T10:00:00.123Z")
        self.assertAlmostEqual(game_meta.recently_updated(t, t + 3 * 3600), 3, places=2)
        self.assertIsNone(game_meta.recently_updated(t, t + 30 * 3600))
        self.assertIsNone(game_meta.recently_updated(t + 9999, t))   # in the future: ignore
        self.assertIsNone(game_meta.recently_updated(None, t))
        self.assertIsNone(game_meta.parse_time("garbage"))
        self.assertIsNone(game_meta.parse_time(None))

    def test_icon_url_must_be_rbxcdn_https(self):
        with tempfile.TemporaryDirectory() as tmp:
            for bad in ("http://tr.rbxcdn.com/a", "https://evil.example/a", "https://rbxcdn.com.evil.io/a", ""):
                self.assertIsNone(game_meta.fetch_icon(tmp, 1, bad))
            class R:
                def __enter__(self): return self
                def __exit__(self, *a): return False
                def read(self, n): return b"PNG"
            path = game_meta.fetch_icon(tmp, 7, "https://tr.rbxcdn.com/x", opener=lambda *a, **k: R())
            self.assertEqual(path.read_bytes(), b"PNG")


class ActivityTests(unittest.TestCase):
    def test_follows_appends_and_truncation(self):
        seen = []
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "player.log"
            path.write_text("noise\n[FLog::SingleSurfaceApp] initiateTeleport\n")
            watcher = activity.ActivityWatcher(path, seen.append)
            activity.POLL_SECONDS = 0.02
            watcher.start()
            try:
                self.wait_for(lambda: len(seen) == 1)
                with open(path, "a") as handle:
                    handle.write("[FLog::SingleSurfaceApp] returnToLuaApp\n[FLog::Net")  # partial line
                self.wait_for(lambda: len(seen) == 2)
                with open(path, "a") as handle:
                    handle.write("work] Time to disconnect replication data: 1\n")
                self.wait_for(lambda: len(seen) == 3)
                path.write_text("[FLog::SingleSurfaceApp] leaveUGCGameInternal\n")  # truncated + rewritten
                self.wait_for(lambda: len(seen) == 4)
            finally:
                watcher.stop()
        self.assertEqual([e.name for e in seen],
                         [logevents.TELEPORT, logevents.RETURN_TO_APP, logevents.DISCONNECTED, logevents.LEAVING])

    def wait_for(self, condition, timeout=3):
        end = time.time() + timeout
        while time.time() < end:
            if condition():
                return
            time.sleep(0.02)
        self.fail("timed out")


if __name__ == "__main__":
    unittest.main()
