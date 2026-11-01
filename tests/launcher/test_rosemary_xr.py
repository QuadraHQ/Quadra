"""Tests for VR under Rosemary: capability check, library build, the environment and
flags a VR start adds, and the fallbacks. A fake rosemary stands in for the real one.

Run: PYTHONPATH=launcher:tests/launcher python3 -m unittest discover -s tests/launcher -v"""
import json
import os
import tempfile
import threading
import time
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

from quadra import core, rosemary_client as client, rosemary_runtime as runtime, rosemary_xr as xr, vr

from test_rosemary import make_zip

FAKE_VR = """#!/bin/sh
# supports ROSEMARY_DYLD_INSERT_FORCE
case "$1" in
  prefix) mkdir -p "$3/.rosemary"; echo "base:" > "$3/.rosemary/state.yaml"; exit 0 ;;
  server) exit 0 ;;
  run) { echo "INSERT=$ROSEMARY_DYLD_INSERT_LIBRARIES"; echo "FORCE=$ROSEMARY_DYLD_INSERT_FORCE"
         echo "ENVIRON=$ROSEMARY_ENVIRON" | tr '\\n' '|'; echo; echo "EGL=$SDL_VIDEO_FORCE_EGL"; } >> "$FAKE_LOG"
       sleep 0.3; exit 0 ;;
esac
"""
FAKE_PLAIN = FAKE_VR.replace("ROSEMARY_DYLD_INSERT_FORCE", "ROSEMARY_NO_OVERRIDE_HERE")


class CapabilityTests(unittest.TestCase):
    def test_only_a_binary_with_the_override_supports_vr(self):
        with tempfile.TemporaryDirectory() as tmp:
            good, plain = Path(tmp) / "good", Path(tmp) / "plain"
            good.write_bytes(b"\x7fELF" + b"x" * 10_000_000 + b"ROSEMARY_DYLD_INSERT_FORCE" + b"y" * 100)
            plain.write_bytes(b"\x7fELF" + b"x" * 1000 + b"ROSEMARY_DYLD_INSERT_LIBRARIES")
            self.assertTrue(xr.supports_vr(good))
            self.assertFalse(xr.supports_vr(plain))
            self.assertFalse(xr.supports_vr(Path(tmp) / "missing"))

    def test_the_name_split_across_a_read_boundary_is_found(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "edge"
            path.write_bytes(b"a" * ((8 << 20) - 5) + b"ROSEMARY_DYLD_INSERT_FORCE")
            self.assertTrue(xr.supports_vr(path))


class BuildTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        root = Path(self.tmp.name)
        (root / "project" / "rosemary_vr").mkdir(parents=True)
        for name in xr.SOURCES:
            (root / "project" / name).write_text("// source")
        (root / "project" / "rosemary_vr" / "build_rosemary_vr.sh").write_text("#!/bin/sh\n")
        self.core = SimpleNamespace(PROJECT=root / "project", CACHE_DIR=root / "cache")
        self.addCleanup(self.tmp.cleanup)

    def runner_that_builds(self, calls):
        def run(argv, env=None, **kwargs):
            calls.append((argv, env["QUADRA_BUILD_DIR"]))
            target = Path(env["QUADRA_BUILD_DIR"]) / "libQuadraVR.dylib"
            target.write_bytes(b"dylib")
            return SimpleNamespace(returncode=0, stdout="", stderr="")
        return run

    def test_builds_when_missing_and_not_again_while_fresh(self):
        calls = []
        path = xr.ensure_dylib(self.core, self.runner_that_builds(calls))
        self.assertEqual(path, self.core.CACHE_DIR / "rosemary-vr" / "libQuadraVR.dylib")
        xr.ensure_dylib(self.core, self.runner_that_builds(calls))
        self.assertEqual(len(calls), 1)
        # a source edited after the build makes it stale
        future = time.time() + 100
        os.utime(self.core.PROJECT / "vr_xr.c", (future, future))
        xr.ensure_dylib(self.core, self.runner_that_builds(calls))
        self.assertEqual(len(calls), 2)

    def test_build_failure_says_why(self):
        def failing(argv, env=None, **kwargs):
            return SimpleNamespace(returncode=1, stdout="", stderr="clang: error: no sysroot\n")
        with self.assertRaisesRegex(xr.VrUnavailable, "no sysroot"):
            xr.ensure_dylib(self.core, failing)

    def test_missing_script_is_a_clear_error(self):
        (self.core.PROJECT / "rosemary_vr" / "build_rosemary_vr.sh").unlink()
        with self.assertRaisesRegex(xr.VrUnavailable, "build script"):
            xr.ensure_dylib(self.core, lambda *a, **k: None)


class PrepareTests(unittest.TestCase):
    def test_vr_off_means_nothing(self):
        self.assertIsNone(xr.prepare({"vr": False}, Path("/r"), core))

    def test_a_rosemary_without_the_override_is_refused_with_a_reason(self):
        with mock.patch.object(xr, "supports_vr", return_value=False):
            with self.assertRaisesRegex(xr.VrUnavailable, "Quadra's own build"):
                xr.prepare({"vr": True}, Path("/r"), core)

    def test_no_openxr_runtime_is_a_reason(self):
        with mock.patch.object(xr, "supports_vr", return_value=True), \
                mock.patch.object(vr, "loader_installed", return_value=True), \
                mock.patch.object(vr, "active_runtime", return_value=None):
            with self.assertRaisesRegex(xr.VrUnavailable, "OpenXR"):
                xr.prepare({"vr": True}, Path("/r"), core)

    def test_session_environment_and_flags(self):
        session = xr.VrSession(Path("/b/libQuadraVR.dylib"), ["QUADRA_VR=openxr", "QUADRA_VR_SCALE=70"], "VR: x")
        env = session.apply({"ROSEMARY_DYLD_INSERT_LIBRARIES": "/other.dylib", "ROSEMARY_ENVIRON": "FOO=bar"})
        self.assertEqual(env["ROSEMARY_DYLD_INSERT_LIBRARIES"], "/b/libQuadraVR.dylib:/other.dylib")
        self.assertEqual(env["ROSEMARY_DYLD_INSERT_FORCE"], "1")
        self.assertEqual(env["ROSEMARY_ENVIRON"].split("\n"), ["FOO=bar", "QUADRA_VR=openxr", "QUADRA_VR_SCALE=70"])
        self.assertEqual(env["SDL_VIDEO_FORCE_EGL"], "1")
        self.assertEqual(session.flags["FFlagDebugGraphicsPreferOpenGL"], "True")
        self.assertEqual(session.flags["FFlagDebugGraphicsDisableMetal"], "True")
        again = session.apply(env)      # applying twice does not duplicate anything
        self.assertEqual(again["ROSEMARY_DYLD_INSERT_LIBRARIES"], "/b/libQuadraVR.dylib:/other.dylib")
        self.assertEqual(again["ROSEMARY_ENVIRON"].count("QUADRA_VR=openxr"), 1)


class ZinkTests(unittest.TestCase):
    def test_zink_adds_mesas_driver_and_egl_vendor(self):
        session = xr.VrSession(Path("/b/x.dylib"), [], "VR: x", zink_vendor="/v/50_mesa.json")
        env = session.apply({})
        self.assertEqual(env["MESA_LOADER_DRIVER_OVERRIDE"], "zink")
        self.assertEqual(env["__EGL_VENDOR_LIBRARY_FILENAMES"], "/v/50_mesa.json")
        plain = xr.VrSession(Path("/b/x.dylib"), [], "VR: x").apply({})
        self.assertNotIn("MESA_LOADER_DRIVER_OVERRIDE", plain)

    def test_vendor_file_discovery(self):
        with tempfile.TemporaryDirectory() as tmp:
            Path(tmp, "10_nvidia.json").write_text("{}")
            Path(tmp, "50_mesa.json").write_text("{}")
            self.assertEqual(xr.mesa_egl_vendor((tmp,)), str(Path(tmp, "50_mesa.json")))
            self.assertIsNone(xr.mesa_egl_vendor((tmp + "/nope",)))

    def test_prepare_with_zink_needs_mesa(self):
        with mock.patch.object(xr, "supports_vr", return_value=True), \
                mock.patch.object(vr, "loader_installed", return_value=True), \
                mock.patch.object(vr, "active_runtime", return_value=Path("/r.json")), \
                mock.patch.object(xr, "ensure_dylib", return_value=Path("/b/x.dylib")):
            with mock.patch.object(xr, "mesa_egl_vendor", return_value=None):
                with self.assertRaisesRegex(xr.VrUnavailable, "zink"):
                    xr.prepare({"vr": True, "rosemary_vr_zink": True}, Path("/r"), core)
            with mock.patch.object(xr, "mesa_egl_vendor", return_value="/v/50_mesa.json"):
                session = xr.prepare({"vr": True, "rosemary_vr_zink": True}, Path("/r"), core)
                self.assertEqual(session.zink_vendor, "/v/50_mesa.json")
                self.assertIn("zink", session.log)


class SessionVrTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.addCleanup(self.tmp.cleanup)
        os.environ["FAKE_LOG"] = str(self.root / "calls.log")
        for patch in (mock.patch.object(client, "data_dir", lambda: self.root / "rosemary"),
                      mock.patch.object(core, "FAST_FLAGS_USER", self.root / "config" / "fastflags.json"),
                      mock.patch.object(core, "FAST_FLAGS", self.root / "none" / "x.json"),
                      mock.patch.object(core, "CONFIG_DIR", self.root / "config"),
                      mock.patch.object(vr, "loader_installed", return_value=True),
                      mock.patch.object(vr, "active_runtime", return_value=Path("/runtime/monado.json")),
                      mock.patch.object(xr, "ensure_dylib", return_value=Path("/b/libQuadraVR.dylib"))):
            patch.start()
            self.addCleanup(patch.stop)
        archive = self.root / "a.zip"
        make_zip(archive)
        client.install(archive, client.Build("version-abc123", "0.745.0.1", "x86_64"), lambda *a: None, threading.Event())
        self.bundle = client.installed_builds()[0][1]

    def fake(self, text):
        path = self.root / "fake-rosemary"
        path.write_text(text)
        path.chmod(0o755)
        return str(path)

    def run_session(self, program, **extra):
        settings = {"rosemary_program": program, "rosemary_prefix": str(self.root / "prefix"), "enable_custom_mods": False,
                    **extra}
        session = runtime.RosemarySession(settings, logs_dir=self.root / "logs")
        session.start()
        end = time.time() + 6
        while time.time() < end and session.poll() is None:
            time.sleep(0.1)
        return session

    def flags(self):
        return json.loads((self.bundle / "Contents/MacOS/ClientSettings/ClientAppSettings.json").read_text())

    def test_vr_start_injects_and_forces_opengl(self):
        core.save_fast_flags({"MyFlag": True})
        session = self.run_session(self.fake(FAKE_VR), vr=True, vr_render_scale=80, vr_mirror=False)
        log = (self.root / "calls.log").read_text()
        self.assertIn("INSERT=/b/libQuadraVR.dylib", log)
        self.assertIn("FORCE=1", log)
        self.assertIn("QUADRA_VR=openxr-headset|QUADRA_VR_SCALE=80|QUADRA_XR_RUNTIME_JSON=/runtime/monado.json|", log)
        self.assertIn("EGL=1", log)
        self.assertEqual(self.flags(), {"MyFlag": True, "FFlagDebugGraphicsPreferOpenGL": "True",
                                        "FFlagDebugGraphicsDisableMetal": "True"})
        self.assertIn("VR: ", session.log_path.read_text())
        self.assertIn("(Rosemary)", session.log_path.read_text())

    def test_vr_flags_do_not_stay_behind_when_vr_is_turned_off(self):
        core.save_fast_flags({"MyFlag": True})
        self.run_session(self.fake(FAKE_VR), vr=True)
        self.run_session(self.fake(FAKE_VR), vr=False)
        self.assertEqual(self.flags(), {"MyFlag": True})
        log = (self.root / "calls.log").read_text().splitlines()
        self.assertEqual(log[-4], "INSERT=")      # nothing injected the second time

    def test_a_rosemary_without_vr_support_still_starts_the_game_and_logs_why(self):
        session = self.run_session(self.fake(FAKE_PLAIN), vr=True)
        self.assertEqual(session.poll(), 0)
        text = session.log_path.read_text()
        self.assertIn("VR: This Rosemary cannot load VR", text)
        self.assertNotIn("INSERT=/b/", (self.root / "calls.log").read_text())
        self.assertEqual(self.flags(), {})
        self.assertTrue(any("cannot load VR" in note for note in session.notes))


if __name__ == "__main__":
    unittest.main()
