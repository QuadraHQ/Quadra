"""Optional UI regression for the warning shown before VR is turned on.

GDK_BACKEND=x11 GSK_RENDERER=cairo xvfb-run -a -s "-screen 0 1280x900x24" python3 tests/vr_warning_ui.py

Settings are never read from or written to disk: load_settings and save_settings are replaced.
"""
from pathlib import Path
import sys
import threading
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "launcher"))
# Suppress unrelated startup update/avatar workers in this UI fixture.
threading.Thread.start = lambda self: None
from quadra import core, ui
from gi.repository import Adw, Gio, Gtk

saved, shown = [], []
application = Adw.Application(application_id="org.quadrahq.Quadra.VRWarningTest",
                              flags=Gio.ApplicationFlags.NON_UNIQUE)
application.register(None)


def walk(widget):
    yield widget
    child = widget.get_first_child()
    while child:
        yield from walk(child)
        child = child.get_next_sibling()


def switch_row(root, title):
    found = [w for w in walk(root) if isinstance(w, Adw.SwitchRow) and w.get_title() == title]
    assert len(found) == 1, (title, len(found))
    return found[0]


def capture(dialog, parent=None):
    shown.append(dialog)


def answer(dialog, choice, hide=False):
    """Press a button of the dialog the way the user would, with the switch in it set first."""
    switch_row(dialog.get_extra_child(), "Don't show this again").set_active(hide)
    dialog.emit("response", choice)


def stored_vr():
    return saved[-1]["vr"] if saved else None


def build(settings):
    with patch.object(core, "load_settings", return_value=dict(settings)):
        return ui.LauncherWindow(application)


with patch.object(core, "save_settings", side_effect=lambda value: saved.append(dict(value))), \
        patch.object(Adw.AlertDialog, "present", capture):
    assert core.DEFAULT_SETTINGS["vr"] is False and core.DEFAULT_SETTINGS["vr_warning_hidden"] is False

    window = build(core.DEFAULT_SETTINGS)
    vr = switch_row(window.settings_page.vr_page, "VR headset (OpenXR)")
    scale = [w for w in walk(window.settings_page.vr_page) if isinstance(w, Adw.SpinRow)][0]
    assert not vr.get_active() and not shown and not scale.get_sensitive()

    # Turning it on asks first. Until the answer, the switch is off and VR is not stored as on.
    vr.set_active(True)
    assert len(shown) == 1 and shown[0].get_heading() == "Turn on VR?"
    body = shown[0].get_body()
    assert "memory" in body and "cheat" in body and "anti-cheat" in body, body
    assert not vr.get_active() and window.settings["vr"] is False and stored_vr() is False
    assert not scale.get_sensitive()

    # Cancel leaves it off, and "Don't show this again" is not kept for a refusal.
    answer(shown[0], "cancel", hide=True)
    assert not vr.get_active() and window.settings["vr"] is False
    assert window.settings["vr_warning_hidden"] is False
    assert all(not value["vr_warning_hidden"] and not value["vr"] for value in saved)

    # Closing the dialog (Escape) counts as cancel.
    vr.set_active(True)
    assert len(shown) == 2
    shown[1].emit("response", "cancel")
    assert not vr.get_active() and window.settings["vr"] is False

    # Going ahead turns it on and stores that, and asks again next time when the box was left off.
    vr.set_active(True)
    assert len(shown) == 3
    answer(shown[2], "enable", hide=False)
    assert vr.get_active() and window.settings["vr"] is True and stored_vr() is True
    assert window.settings["vr_warning_hidden"] is False and scale.get_sensitive()
    assert len(shown) == 3
    vr.set_active(False)
    assert window.settings["vr"] is False and stored_vr() is False and not scale.get_sensitive()
    assert len(shown) == 3, "turning VR off never asks"

    # Going ahead with the box on keeps it, and the warning no longer appears.
    vr.set_active(True)
    assert len(shown) == 4
    answer(shown[3], "enable", hide=True)
    assert vr.get_active() and window.settings["vr"] is True
    assert window.settings["vr_warning_hidden"] is True and saved[-1]["vr_warning_hidden"] is True
    vr.set_active(False)
    vr.set_active(True)
    assert len(shown) == 4, "no warning once it was hidden"
    assert vr.get_active() and window.settings["vr"] is True and stored_vr() is True
    window.destroy()

    # Opening the launcher with VR already on, or the warning hidden, shows nothing.
    shown.clear()
    window = build({**core.DEFAULT_SETTINGS, "vr": True})
    assert switch_row(window.settings_page.vr_page, "VR headset (OpenXR)").get_active() and not shown
    window.destroy()
    window = build({**core.DEFAULT_SETTINGS, "vr_warning_hidden": True})
    again = switch_row(window.settings_page.vr_page, "VR headset (OpenXR)")
    again.set_active(True)
    assert again.get_active() and window.settings["vr"] is True and not shown
    window.destroy()

print("PASS: VR warning: asks before turning on, cancel and close keep it off, don't-show-again kept only on go-ahead")
