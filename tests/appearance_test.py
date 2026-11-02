"""Saved appearance choices stay compatible with older settings; GNOME is the default."""
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from quadra import core


class AppearanceSettingsTests(unittest.TestCase):
    def test_settings_without_an_appearance_choose_gnome_without_losing_existing_preferences(self):
        with tempfile.TemporaryDirectory() as directory:
            file = Path(directory) / "settings.json"
            file.write_text(json.dumps({"renderer": "vulkan", "mouse_sensitivity": 1.75}))
            with patch.object(core, "SETTINGS_FILE", file):
                settings = core.load_settings()
                self.assertEqual(settings["launcher_appearance"], "gnome")
                self.assertEqual(settings["renderer"], "vulkan")
                self.assertEqual(settings["mouse_sensitivity"], 1.75)

    def test_a_saved_grid_choice_is_not_moved_to_the_new_default(self):
        with tempfile.TemporaryDirectory() as directory:
            file = Path(directory) / "settings.json"
            file.write_text(json.dumps({"launcher_appearance": "grid"}))
            with patch.object(core, "SETTINGS_FILE", file):
                self.assertEqual(core.load_settings()["launcher_appearance"], "grid")

    def test_each_appearance_persists_and_invalid_values_use_gnome(self):
        with tempfile.TemporaryDirectory() as directory:
            file = Path(directory) / "settings.json"
            with patch.object(core, "SETTINGS_FILE", file):
                for code in ("grid", "gnome", "monochrome"):
                    core.save_settings(dict(core.DEFAULT_SETTINGS, launcher_appearance=code))
                    self.assertEqual(core.load_settings()["launcher_appearance"], code)
                for code in (None, True, 2, [], {}, "unknown", "../../style.css"):
                    file.write_text(json.dumps({"launcher_appearance": code}))
                    self.assertEqual(core.load_settings()["launcher_appearance"], "gnome")
