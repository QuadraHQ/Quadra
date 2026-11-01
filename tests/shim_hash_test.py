"""The launcher rebuilds the shim when its sources change, so the hash must cover every folder they live in.

Run: PYTHONPATH=launcher python3 -m unittest tests/shim_hash_test.py"""
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from quadra import core


class ShimSourceHashTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        for name in ("darling/shim/libQuadraShims.m", "darling/shim/net_trace.c", "darling/shim/worker_wake.h",
                     "darling/frameworks/CoreML.m", "vr/vr_device.c", "vr/vr_xr.h"):
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("// original")
        script = self.root / "build_debug_shim.sh"
        script.write_text("#!/bin/sh")
        for patcher in (mock.patch.object(core, "PROJECT", self.root), mock.patch.object(core, "BUILD_SCRIPT", script)):
            patcher.start()
            self.addCleanup(patcher.stop)

    def test_every_source_folder_and_the_build_script_change_the_hash(self):
        before = core._shim_sources_hash()
        self.assertEqual(core._shim_sources_hash(), before)
        for name in ("darling/shim/libQuadraShims.m", "darling/shim/net_trace.c", "darling/shim/worker_wake.h",
                     "darling/frameworks/CoreML.m", "vr/vr_device.c", "vr/vr_xr.h", "build_debug_shim.sh"):
            with self.subTest(name=name):
                path = self.root / name
                original = path.read_text()
                path.write_text(original + "\n// edited")
                self.assertNotEqual(core._shim_sources_hash(), before, f"editing {name} did not change the hash")
                path.write_text(original)
                self.assertEqual(core._shim_sources_hash(), before)

    def test_the_real_repository_sources_are_all_found(self):
        with mock.patch.object(core, "PROJECT", Path(__file__).resolve().parents[1]):
            sources = [*(core.PROJECT / "darling" / "shim").glob("*"), *(core.PROJECT / "vr").glob("*"),
                       *(core.PROJECT / "darling" / "frameworks").glob("*")]
        names = {path.name for path in sources}
        for expected in ("libQuadraShims.m", "net_trace.c", "vr_device.c", "vr_xr.c", "CoreML.m"):
            self.assertIn(expected, names)


if __name__ == "__main__":
    unittest.main()
