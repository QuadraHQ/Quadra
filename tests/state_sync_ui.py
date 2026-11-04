from pathlib import Path
import sys
import threading
import time
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "launcher"))
threading.Thread.start = lambda self: None
from quadra import core, ui
from quadra.rosemary import runtime as rosemary_runtime
from gi.repository import Adw, Gio, GLib

saved, toasts, dialogs = [], [], []
application = Adw.Application(application_id="org.quadrahq.Quadra.StateSyncTest",
                              flags=Gio.ApplicationFlags.NON_UNIQUE)
application.register(None)


def walk(widget):
    yield widget
    child = widget.get_first_child()
    while child:
        yield from walk(child)
        child = child.get_next_sibling()


def rows(root, kind, title):
    return [widget for widget in walk(root) if isinstance(widget, kind) and widget.get_title() == title]


def switches(root):
    return [widget for widget in walk(root) if isinstance(widget, Adw.SwitchRow)]


def settle(seconds=0.2):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        while GLib.MainContext.default().iteration(False):
            pass
        time.sleep(.01)


settings = dict(core.DEFAULT_SETTINGS, setup_complete=True, runtime_backend="darling",
                auto_check_roblox_updates=False, auto_check_ciderdarling_updates=False)
with patch.object(core, "load_settings", return_value=settings), \
        patch.object(core, "save_settings", side_effect=lambda value: saved.append(dict(value))), \
        patch.object(core, "installed_version", return_value="0.700.0.7000000") as installed, \
        patch.object(core, "signed_in", return_value=True), \
        patch.object(core, "darlingserver_running", return_value=False), \
        patch.object(core, "delete_roblox") as delete_roblox, \
        patch.object(rosemary_runtime, "installed_version", return_value=None), \
        patch.object(rosemary_runtime, "rosemary_ready", return_value=False), \
        patch.object(rosemary_runtime, "running", return_value=False), \
        patch.object(ui.LauncherWindow, "_periodic_ciderdarling_updates", return_value=False), \
        patch.object(ui, "_toast", side_effect=lambda _overlay, text: toasts.append(text)), \
        patch.object(Adw.AlertDialog, "present", lambda dialog, _parent=None: dialogs.append(dialog)):
    window = ui.LauncherWindow(application)
    page, play = window.settings_page, window.play_page

    mangohud, = rows(page, Adw.SwitchRow, "MangoHud overlay")
    window.set_setting("mangohud", True)
    assert mangohud.get_active() and saved[-1]["mangohud"] is True
    raw_mouse, = rows(page, Adw.SwitchRow, "Raw mouse input")
    window.set_setting("raw_mouse", False)
    assert not raw_mouse.get_active()

    debugger = rows(page, Adw.SwitchRow, "Allow freeze diagnostics")
    assert len(debugger) == 2
    debugger[0].set_active(True)
    assert debugger[1].get_active() and window.settings["allow_debugger"] is True
    saves = len(saved)
    debugger[1].set_active(True)
    assert len(saved) == saves, "an unchanged value is not saved again"

    window.set_setting("roblox_channel", "zbeta")
    assert page.channel_row.get_subtitle() == "zbeta"

    window.set_setting("discord_rpc", False)
    game_name, = rows(page, Adw.SwitchRow, "Show experience name in Discord")
    assert not game_name.get_sensitive()
    window.set_setting("discord_rpc", True)
    assert game_name.get_sensitive()

    assert play.play.get_label() == "Play"
    window.set_setting("runtime_backend", "rosemary")
    assert page.backend_row.get_selected() == 1
    assert play.play.get_label() == "Install Roblox"
    assert play.dashboard_facts["runtime"].get_label() == "Rosemary"
    assert play.dashboard_facts["renderer"].get_label() == "Metal"
    window.set_setting("rosemary_force_opengl", True)
    assert play.dashboard_facts["renderer"].get_label() == "OpenGL"
    window.set_setting("rosemary_force_opengl", False)
    window.set_setting("vr", True)
    assert play.dashboard_facts["renderer"].get_label() == "OpenGL"
    window.set_setting("rosemary_vr_zink", True)
    assert play.dashboard_facts["renderer"].get_label() == "Vulkan / Zink"
    window.set_setting("vr", False)
    assert play.dashboard_facts["renderer"].get_label() == "Metal"
    window.set_setting("runtime_backend", "darling")
    assert page.backend_row.get_selected() == 0 and play.play.get_label() == "Play"

    window.set_setting("renderer", "vulkan")
    assert play.dashboard_facts["renderer"].get_label() == "Vulkan / Zink"
    window.set_setting("renderer", "opengl")

    window.busy = "updating"
    assert not play.play.get_sensitive()
    page._install_rosemary_client()
    assert toasts[-1] == "Please wait, the launcher is busy"
    page.delete_roblox()
    dialogs[-1].emit("response", "delete")
    delete_roblox.assert_not_called()
    window.busy = None
    assert play.play.get_sensitive()

    installed.return_value = "0.701.0.7010000"
    window.client_changed()
    assert page.version_row.get_subtitle() == "0.701.0.7010000"
    assert play.dashboard_facts["client"].get_label() == "0.701.0.7010000"

    page.delete_roblox()
    dialogs[-1].emit("response", "delete")
    delete_roblox.assert_called_once()
    assert window.busy is None

    window.set_setting("show_playtime", True)
    window.settings.stage("playtime_seconds", 3600)
    assert play.playtime_label.get_text() == "1H"
    window.set_setting("show_playtime", False)
    assert not play.playtime_box.get_visible()

    mangohud_position = switches(page).index(mangohud)
    window.set_language("ru")
    settle()
    assert window.settings_page is not page and window.play_page is not play
    rebuilt_mangohud = switches(window.settings_page)[mangohud_position]
    assert rebuilt_mangohud.get_title() != mangohud.get_title() and rebuilt_mangohud.get_active()
    window.set_setting("mangohud", False)
    assert not rebuilt_mangohud.get_active()
    assert mangohud.get_active(), "a page from before the rebuild no longer follows the settings"
    window.studio_status.set((None, ""))
    assert not window.play_page.studio.get_sensitive() and play.studio.get_sensitive()
    window.studio_status.set(None)
    window.set_language("en")
    settle()
    window.destroy()

print("PASS: rows, pages and the busy lock follow one settings store; rebuilt pages drop old subscriptions")
