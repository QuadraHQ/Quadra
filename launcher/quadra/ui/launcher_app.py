"""The GTK application and the launcher's main()."""

import os
import signal
import threading

from gi.repository import Adw, Gdk, Gio, GLib, Gtk

from .. import core, tray
from .. import uri as uri_handoff
from ..i18n import _
from . import launcher_window

__all__ = [
    "APP_ID",
    "LauncherApp",
    "main",
]

APP_ID = "org.quadrahq.Quadra"


class LauncherApp(Adw.Application):
    def __init__(self, page=None, open_settings=False):
        super().__init__(
            application_id=APP_ID, flags=Gio.ApplicationFlags.DEFAULT_FLAGS
        )
        self.window = None
        self.page = page
        self.open_settings = open_settings
        self.tray = None
        self._stopping = False
        # `quadrahq --settings` (the shortcut's "Settings" action) opens the
        # settings window of a running Quadra.
        settings_action = Gio.SimpleAction.new("open-settings", None)
        settings_action.connect("activate", lambda *_args: self.show_settings())
        self.add_action(settings_action)
        # `quadrahq --ciderdarling-settings` activates this in a running launcher.
        action = Gio.SimpleAction.new("ciderdarling-settings", None)
        action.connect("activate", lambda *_args: self._show_ciderdarling_settings())
        self.add_action(action)

    def show_settings(self):
        if not self.window:
            self.open_settings = True
            self.activate()
            return
        self.window.present_settings()

    def apply_tray(self):
        """Start or stop the tray icon to match the setting."""
        wanted = bool(self.window and self.window.settings.get("tray_icon", True))
        if wanted and self.tray is None and not os.environ.get("QUADRA_NO_TRAY"):
            tray_item = tray.Tray(
                "Quadra",
                "quadrahq",
                core.ICONS / "quadrahq-64.png",
                self.show_settings,
                self._tray_entries,
            )
            if tray_item.start():
                self.tray = tray_item
        elif not wanted and self.tray is not None:
            self.tray.stop()
            self.tray = None

    def _tray_entries(self):
        window = self.window
        running = bool(window and window.session)
        return [
            (_("Open settings"), self.show_settings, True),
            (_("Stop Roblox"), lambda: window.stop(), running),
            None,
            (_("Quit Quadra"), lambda: self.stop_from_signal(signal.SIGTERM), True),
        ]

    def do_shutdown(self):
        if self.tray is not None:
            self.tray.stop()
            self.tray = None
        Adw.Application.do_shutdown(self)

    def _show_ciderdarling_settings(self):
        if not self.window:
            self.page = "ciderdarling"
            self.activate()
            return
        self.window.set_visible(True)
        self.window.present()
        if not self.window.setup_active:
            self.window.show_ciderdarling_settings()

    def stop_from_signal(self, signal_number):
        """SIGTERM/SIGINT/SIGHUP: end the game session like a normal exit
        (its audio FIFOs, DNS forwarder and Roblox processes), then quit."""
        if self._stopping:
            return GLib.SOURCE_CONTINUE
        self._stopping = True
        window = self.window
        session = window.session if window else None
        if window:
            # The game watcher may queue a pending join as the game ends.
            window.stopping = True
        print(f"Signal {signal_number}: closing Quadra")

        def finish():
            try:
                if session:
                    session.finish()
            except Exception as error:
                print("Cleaning up the game failed:", error)
            GLib.idle_add(self._quit_after_signal)

        threading.Thread(target=finish, daemon=True).start()
        return GLib.SOURCE_CONTINUE

    def _quit_after_signal(self):
        window = self.window
        if window:
            try:
                window.settings_page.flush()
                window._stop_web()
                window._stop_rpc()
                core.save_settings(window.settings)
            except Exception as error:
                print("Saving before exit failed:", error)
        self.quit()
        return False

    def do_activate(self):
        pending = uri_handoff.peek_pending()
        if not self.window:
            Gtk.Window.set_default_icon_name("quadrahq")
            Gtk.IconTheme.get_for_display(Gdk.Display.get_default()).add_search_path(
                str(core.PROJECT / "launcher" / "icons")
            )
            Gtk.IconTheme.get_for_display(Gdk.Display.get_default()).add_search_path(
                str(core.ICONS)
            )
            # Audio FIFOs of a launcher that was killed (HostAudio.stop never ran).
            core.sweep_audio_fifos()
            self.window = launcher_window.LauncherWindow(self, self.page)
            # Keep running while the window is hidden during a game.
            self.hold()
            self.window.connect("close-request", self._close)
            self.apply_tray()
            # Roblox starts straight away behind a splash; the settings window
            # only opens when asked for (or when something needs the user:
            # setup, a missing client, an error).
            needs_window = (
                self.open_settings
                or self.page is not None
                or self.window.setup_active
                or not self.window.runtime_ready()
                or os.environ.get("QUADRA_PAGE")
            )
            if needs_window:
                self.window.present_settings()
            else:
                self.window.autostart()
            if os.environ.get("QUADRA_PAGE"):
                # Screenshots: no focused field.
                GLib.timeout_add(300, lambda: self.window.set_focus(None) and False)
            return
        # Activated again while running (the shortcut, a browser link).
        if pending:
            GLib.idle_add(self.window.handle_uri, pending)
        else:
            self.window.present_settings()

    def _close(self, window):
        window.settings_page.flush()
        if window.session or window.busy:
            # Closing during a game only hides the launcher. Closing during an
            # update, build or start hides it until that is done: quitting
            # would stop the work half way (a half unpacked client).
            window.set_visible(False)
            window.quit_when_idle = window.busy is not None
            return True
        self.release()
        self.quit()
        return False


def main(page=None, open_settings=False):
    # Some desktops set GTK's old dark-theme preference globally. Libadwaita
    # owns the color scheme; clear the legacy property for this process before
    # constructing Adw.Application, without editing the user's GTK settings.
    Gtk.init()
    settings = Gtk.Settings.get_default()
    if settings and settings.get_property("gtk-application-prefer-dark-theme"):
        settings.set_property("gtk-application-prefer-dark-theme", False)
    application = LauncherApp(page, open_settings)
    if open_settings:
        # One launcher at a time: a running one opens its settings itself.
        try:
            application.register(None)
        except GLib.Error as error:
            print("Could not register Quadra:", error)
        if application.get_is_remote():
            application.activate_action("open-settings", None)
            connection = application.get_dbus_connection()
            if connection:
                connection.flush_sync(None)
            return 0
    if page == "ciderdarling":
        # One launcher at a time: a running one opens the page itself.
        try:
            application.register(None)
        except GLib.Error as error:
            print("Could not register Quadra:", error)
        if application.get_is_remote():
            application.activate_action("ciderdarling-settings", None)
            connection = application.get_dbus_connection()
            if connection:
                connection.flush_sync(None)
            return 0
    for number in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP):
        if number == signal.SIGHUP and signal.getsignal(number) == signal.SIG_IGN:
            continue  # started with nohup: closing the terminal must not end the game
        GLib.unix_signal_add(
            GLib.PRIORITY_DEFAULT, number, application.stop_from_signal, number
        )
    return application.run(None)
