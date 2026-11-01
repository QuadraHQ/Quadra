"""Tests for the Rosemary parity work: user flags apart from Darling's, Rosemary's
options, CPU limit at start, mods per build, rollback, sign-out, runtime table.

Run: PYTHONPATH=launcher python3 -m unittest discover -s tests/launcher -v"""
import json
import os
import tempfile
import threading
import time
import unittest
from pathlib import Path
from unittest import mock

from quadra import backends, core, mods, rosemary_client as client, rosemary_runtime as runtime

from test_rosemary import FAKE, make_zip


class FlagTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        root = Path(self.tmp.name)
        self.bundle_file = root / "bundle" / "ClientAppSettings.json"
        for patch in (mock.patch.object(core, "FAST_FLAGS", self.bundle_file),
                      mock.patch.object(core, "FAST_FLAGS_USER", root / "config" / "fastflags.json")):
            patch.start()
            self.addCleanup(patch.stop)
        self.addCleanup(self.tmp.cleanup)

    def test_first_load_carries_the_users_flags_without_darlings(self):
        self.bundle_file.parent.mkdir(parents=True)
        self.bundle_file.write_text(json.dumps({**core.DARLING_FLAGS, "DFIntTaskSchedulerTargetFps": 144}))
        self.assertEqual(core.load_fast_flags(), {"DFIntTaskSchedulerTargetFps": 144})

    def test_save_keeps_users_flags_and_gives_darling_both(self):
        self.bundle_file.parent.mkdir(parents=True)
        core.save_fast_flags({"A": 1})
        self.assertEqual(core.load_fast_flags(), {"A": 1})                      # nothing of Darling's in the user's file
        written = json.loads(self.bundle_file.read_text())
        self.assertEqual(written["A"], 1)
        self.assertEqual(written["FFlagUseRbxTransportClient"], "False")        # Darling's own flags are in its copy

    def test_save_without_a_darling_client_still_works(self):
        core.save_fast_flags({"B": 2})
        self.assertEqual(core.load_fast_flags(), {"B": 2})
        self.assertFalse(self.bundle_file.exists())

    def test_user_value_of_a_darling_flag_is_kept_in_the_user_file(self):
        core.save_fast_flags({"DFFlagHttpLocalThrottle": "True"})
        self.assertEqual(core.load_fast_flags()["DFFlagHttpLocalThrottle"], "True")


class OptionTests(unittest.TestCase):
    def test_refresh_rate_validation(self):
        self.assertEqual(runtime.refresh_rate("144"), "144")
        self.assertEqual(runtime.refresh_rate(" 59.94 "), "59.94")
        for bad in ("", "0", "-1", "abc", "nan", "inf", "1_0"):
            self.assertEqual(runtime.refresh_rate(bad), "", bad)

    def test_options_become_variables_and_bad_values_are_dropped(self):
        env = runtime.run_env({"rosemary_presentation_mode": "mailbox", "rosemary_decorations": "server",
                               "rosemary_menu": "strip", "rosemary_command_key": "ctrl",
                               "rosemary_refresh_rate": "165", "rosemary_confine_pointer": True,
                               "rosemary_control_edit_keys": True, "rosemary_plain_ctrl_click": True}, {})
        self.assertEqual(env["ROSEMARY_METAL_PRESENTATION_MODE"], "mailbox")
        self.assertEqual((env["ROSEMARY_DECORATIONS"], env["ROSEMARY_MENU"], env["ROSEMARY_COMMAND_KEY"]),
                         ("server", "strip", "ctrl"))
        self.assertEqual(env["ROSEMARY_SCREEN_REFRESH_RATE"], "165")
        for name in ("ROSEMARY_CONFINE_POINTER", "ROSEMARY_CONTROL_EDIT_KEYS", "ROSEMARY_PLAIN_CTRL_CLICK"):
            self.assertEqual(env[name], "1")
        bad = runtime.run_env({"rosemary_presentation_mode": "bogus", "rosemary_menu": "x", "rosemary_refresh_rate": "nope"}, {})
        self.assertNotIn("ROSEMARY_METAL_PRESENTATION_MODE", bad)
        self.assertNotIn("ROSEMARY_MENU", bad)
        self.assertNotIn("ROSEMARY_SCREEN_REFRESH_RATE", bad)

    def test_inherited_managed_variables_are_cleared(self):
        env = runtime.run_env({}, {"ROSEMARY_MENU": "none", "ROSEMARY_COMMAND_KEY": "alt", "HOME": "/h"})
        self.assertNotIn("ROSEMARY_MENU", env)
        self.assertNotIn("ROSEMARY_COMMAND_KEY", env)
        self.assertEqual(env["HOME"], "/h")


class SessionTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.addCleanup(self.tmp.cleanup)
        self.fake = self.root / "fake-rosemary"
        self.fake.write_text(FAKE.replace('run) echo "game output";', 'run) grep Cpus_allowed_list /proc/self/status >> "$FAKE_LOG"; echo "game output";'))
        self.fake.chmod(0o755)
        os.environ["FAKE_LOG"] = str(self.root / "calls.log")
        for patch in (mock.patch.object(client, "data_dir", lambda: self.root / "rosemary"),
                      mock.patch.dict(os.environ, {"XDG_DATA_HOME": str(self.root / "xdg")}),
                      mock.patch.object(core, "FAST_FLAGS_USER", self.root / "config" / "fastflags.json"),
                      mock.patch.object(core, "FAST_FLAGS", self.root / "none" / "x.json"),
                      mock.patch.object(core, "CONFIG_DIR", self.root / "config"),
                      mock.patch.object(mods, "MODS_DIR", self.root / "mods")):
            patch.start()
            self.addCleanup(patch.stop)
        self.settings = {"rosemary_program": str(self.fake), "rosemary_prefix": str(self.root / "prefix"),
                         "enable_custom_mods": True}

    def install(self, guid="version-abc123", content=None):
        archive = self.root / f"{guid}.zip"
        make_zip(archive, [("RobloxPlayer.app/Contents/Resources/content/sounds/ouch.ogg", content or b"original")])
        client.install(archive, client.Build(guid, "0.745.0.1", "x86_64"), lambda *a: None, threading.Event())

    def run_to_end(self, **extra):
        settings = {**self.settings, **extra}
        session = runtime.RosemarySession(settings, logs_dir=self.root / "logs")
        session.start()
        end = time.time() + 6
        while time.time() < end and session.poll() is None:
            time.sleep(0.1)
        return session

    def test_cpu_limit_is_applied_at_start_and_children_inherit(self):
        self.install()
        allowed = sorted(os.sched_getaffinity(0))
        if len(allowed) < 2:
            self.skipTest("needs two cores")
        session = self.run_to_end(roblox_cpu_limit=1)
        self.assertTrue(session.cpu_limited)
        log = (self.root / "calls.log").read_text()
        self.assertIn(f"Cpus_allowed_list:\t{allowed[0]}\n", log)

    def test_the_client_log_is_the_launch_log(self):
        self.install()
        session = self.run_to_end()
        self.assertEqual(session.game_log(), session.log_path)
        self.assertIn("game output", session.game_log().read_text())

    def test_finish_leaves_no_process_behind(self):
        self.install()
        script = self.root / "slow-rosemary"
        script.write_text("#!/bin/sh\ncase \"$1\" in prefix) mkdir -p \"$3/.rosemary\"; echo base: > \"$3/.rosemary/state.yaml\";; run) sleep 60 & sleep 60;; esac\n")
        script.chmod(0o755)
        session = runtime.RosemarySession({**self.settings, "rosemary_program": str(script)}, logs_dir=self.root / "logs")
        session.start()
        time.sleep(0.4)
        group = session.process.pid
        started = time.time()
        session.finish()
        self.assertLess(time.time() - started, 9)
        with self.assertRaises(ProcessLookupError):
            os.killpg(group, 0)

    def test_no_limit_by_default(self):
        self.install()
        self.assertFalse(self.run_to_end().cpu_limited)

    def test_flags_reach_this_client_from_the_users_file(self):
        self.install()
        core.save_fast_flags({"MyFlag": True})
        self.run_to_end()
        bundle = client.installed_builds()[0][1]
        written = json.loads((bundle / "Contents/MacOS/ClientSettings/ClientAppSettings.json").read_text())
        self.assertEqual(written, {"MyFlag": True})        # none of Darling's flags

    def test_mods_apply_per_build_and_restore_cleanly(self):
        self.install()
        (self.root / "mods" / "sounds").mkdir(parents=True)
        (self.root / "mods" / "sounds" / "ouch.ogg").write_bytes(b"modded")
        self.run_to_end()
        bundle = client.installed_builds()[0][1]
        ouch = bundle / "Contents/Resources/content/sounds/ouch.ogg"
        self.assertEqual(ouch.read_bytes(), b"modded")
        backup = self.root / "rosemary" / "mods_backup" / "version-abc123-x86_64" / "sounds" / "ouch.ogg"
        self.assertEqual(backup.read_bytes(), b"original")
        # the mod goes away: the next start restores this build's original
        (self.root / "mods" / "sounds" / "ouch.ogg").unlink()
        self.run_to_end()
        self.assertEqual(ouch.read_bytes(), b"original")

    def test_mods_off_leaves_the_client_alone(self):
        self.install()
        (self.root / "mods" / "sounds").mkdir(parents=True)
        (self.root / "mods" / "sounds" / "ouch.ogg").write_bytes(b"modded")
        self.run_to_end(enable_custom_mods=False)
        bundle = client.installed_builds()[0][1]
        self.assertEqual((bundle / "Contents/Resources/content/sounds/ouch.ogg").read_bytes(), b"original")


class RollbackTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.addCleanup(self.tmp.cleanup)
        patch = mock.patch.object(client, "data_dir", lambda: self.root / "rosemary")
        patch.start()
        self.addCleanup(patch.stop)

    def install(self, guid, age):
        archive = self.root / f"{guid}.zip"
        make_zip(archive)
        client.install(archive, client.Build(guid, "0.7", "x86_64"), lambda *a: None, threading.Event())
        os.utime(client.versions_dir() / f"{guid}-x86_64", (time.time() - age, time.time() - age))

    def test_remove_old_keeps_one_previous_build(self):
        for guid, age in (("version-a1", 300), ("version-b2", 200), ("version-c3", 100)):
            self.install(guid, age)
        client.remove_old("version-c3-x86_64", keep_previous=1)
        self.assertEqual(sorted(p.name for p in client.versions_dir().iterdir()),
                         ["version-b2-x86_64", "version-c3-x86_64"])

    def test_pinned_build_runs_and_survives_cleanup(self):
        self.install("version-a1", 300)
        self.install("version-b2", 100)
        keys = {b.key for b, _ in client.installed_builds()}
        self.assertEqual(keys, {"version-a1-x86_64", "version-b2-x86_64"})
        settings = {"rosemary_pinned_build": "version-a1-x86_64"}
        self.assertEqual(runtime.installed(settings)[0].key, "version-a1-x86_64")
        self.assertEqual(runtime.previous_build(settings)[0].key, "version-b2-x86_64")
        client.remove_old("version-b2-x86_64", keep_previous=0, pinned="version-a1-x86_64")
        self.assertTrue((client.versions_dir() / "version-a1-x86_64").exists())

    def test_unknown_pin_falls_back_to_newest(self):
        self.install("version-a1", 100)
        self.assertEqual(runtime.installed({"rosemary_pinned_build": "version-zz-x86_64"})[0].key, "version-a1-x86_64")


class MiscTests(unittest.TestCase):
    def test_sign_out_removes_only_the_cookie_store(self):
        with tempfile.TemporaryDirectory() as tmp:
            prefix = Path(tmp)
            cookies = prefix / "root/Users/me/Library/Cookies"
            cookies.mkdir(parents=True)
            (cookies / "com.roblox.RobloxPlayer.cookies.tsv").write_text("secret")
            (cookies / "other.tsv").write_text("keep")
            self.assertEqual(runtime.sign_out(prefix), 1)
            self.assertFalse((cookies / "com.roblox.RobloxPlayer.cookies.tsv").exists())
            self.assertTrue((cookies / "other.tsv").exists())
            self.assertEqual(runtime.sign_out(prefix), 0)

    def test_runtime_table(self):
        self.assertTrue(backends.supports("darling", "dns"))
        self.assertFalse(backends.supports("rosemary", "dns"))
        self.assertTrue(backends.supports("rosemary", "mods"))
        self.assertFalse(backends.supports("rosemary", "vr"))
        self.assertEqual(backends.other_runtimes("dns"), ["Darling"])
        self.assertFalse(backends.supports("nothing", "mods"))


if __name__ == "__main__":
    unittest.main()
