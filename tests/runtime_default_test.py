"""Which runtime a launcher starts on: Rosemary for a new install, Darling for an install that already ran it.

Run: PYTHONPATH=launcher python3 -m unittest tests/runtime_default_test.py"""
import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from quadra import core


class RuntimeDefaultTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.file = Path(self.tmp.name) / "settings.json"
        patcher = mock.patch.object(core, "SETTINGS_FILE", self.file)
        patcher.start()
        self.addCleanup(patcher.stop)
        bundled = mock.patch.object(core, "is_bundled", return_value=False)
        self.bundled = bundled.start()
        self.addCleanup(bundled.stop)

    def backend(self, text=None):
        if text is not None:
            self.file.write_text(text)
        return core.load_settings()["runtime_backend"]

    def test_the_default_is_rosemary(self):
        self.assertEqual(core.DEFAULT_SETTINGS["runtime_backend"], "rosemary")

    def test_a_new_install_starts_on_rosemary(self):
        self.assertFalse(self.file.exists())
        self.assertEqual(self.backend(), "rosemary")
        self.assertEqual(self.backend("{}"), "rosemary")

    def test_a_damaged_file_is_not_an_install_to_keep(self):
        self.assertEqual(self.backend("not json"), "rosemary")
        self.assertEqual(self.backend("[1, 2]"), "rosemary")

    def test_settings_saved_before_the_choice_existed_stay_on_darling(self):
        self.assertEqual(self.backend(json.dumps({"vr": False, "renderer": "opengl"})), "darling")

    def test_a_saved_choice_wins(self):
        self.assertEqual(self.backend(json.dumps({"runtime_backend": "rosemary", "vr": True})), "rosemary")
        self.assertEqual(self.backend(json.dumps({"runtime_backend": "darling"})), "darling")

    def test_other_settings_still_load_over_the_defaults(self):
        self.file.write_text(json.dumps({"runtime_backend": "rosemary", "vr": True, "keep_logs": 7}))
        settings = core.load_settings()
        self.assertTrue(settings["vr"])
        self.assertEqual(settings["keep_logs"], 7)
        self.assertEqual(settings["dns"], core.DEFAULT_SETTINGS["dns"])

    def test_a_bundled_package_stays_on_darling_unless_told_otherwise(self):
        self.bundled.return_value = True
        self.assertEqual(self.backend(), "darling")
        self.assertEqual(self.backend(json.dumps({"runtime_backend": "rosemary"})), "rosemary")


if __name__ == "__main__":
    unittest.main()
