"""The launcher window: it holds the pages, and starts, watches and stops the game."""

import os
import threading
import time

from gi.repository import Adw, Gio, GLib, Gtk

from .. import (
    __version__,
    activity,
    backends,
    core,
    discord,
    fflag_allowlist,
    game_meta,
    history,
    i18n,
    logevents,
    modguard,
    mods,
    region,
    studio,
    theme,
)
from .. import uri as uri_handoff
from ..darling import runtime, runtime_install, runtime_updates
from ..i18n import _
from ..rosemary import runtime as rosemary_runtime
from ..setup import SetupWizard
from . import (
    boot_splash,
    dialogs,
    info_page,
    mods_page,
    play_page,
    servers_page,
    settings_page,
    studio_page,
    system,
    widgets,
)

__all__ = [
    "LauncherWindow",
]


class LauncherWindow(Adw.ApplicationWindow):
    def __init__(self, app, page=None):
        super().__init__(application=app, title="Quadra")
        self.set_default_size(760, 580)
        self.set_resizable(True)
        self.settings = core.SettingsStore(
            core.load_settings(), save=lambda values: core.save_settings(values)
        )
        appearance = self.settings.get("launcher_appearance", "gnome")
        theme.install(appearance)
        if appearance != "gnome":
            self.set_default_size(960, 680)
        i18n.set_language(self.settings.get("language", "en"))
        self.activity_changes = core.Subscribers()
        self.client_changes = core.Subscribers()
        self.studio_status = core.ObservableValue()
        self._page_subscriptions = []
        self._session = None
        # One long operation at a time: starting, updating, building,
        # restarting Darling or signing out. They share the client and Darling.
        self._busy = None
        self.rpc = None
        self.web = None
        self.game_tracker = None
        self.current_game_info = None
        self.game_started_at = None
        self._playtime_clock = 0.0
        self._playtime_before_session = 0
        self._playtime_saved_at = 0.0
        self._rpc_retry_at = 0.0
        self._watch_inflight = None
        self._activity_join = None
        self._shader_preload_session = None
        self._shader_preload_dialog = None
        self._update_required_dialog_active = False
        self.stopping = False
        self._transport_warning_session = None
        self._transport_warning_dialog = None
        self.runtime_install_job = None
        self.cider_release = (
            None if runtime.is_bundled() else runtime_updates.cached(core.CACHE_DIR)
        )
        self.cider_pin = self._cider_pin_for(self.cider_release)
        self.cider_update_checking = False
        self._notified_cider_revision = None
        self.quit_when_idle = False  # the window was closed during an operation
        self.last_log = self._find_last_log()
        self._pending_uri = None
        self._pending_uri_at = 0.0
        self.diagnostics_running = False
        self.setup_active = False
        self._notified_server_ip = None
        self.activity = None
        self.splash = None
        self.settings_open = False
        self.runtime_widgets = []
        self.launch_cancelled = False
        self.play_page = None
        self.settings_page = None
        self.studio_page = None
        setup_action = Gio.SimpleAction.new("setup", None)
        setup_action.connect("activate", lambda *_args: self.show_setup())
        self.add_action(setup_action)
        diagnostics_action = Gio.SimpleAction.new("collect-diagnostics", None)
        diagnostics_action.connect(
            "activate", lambda *_args: self.collect_diagnostics()
        )
        self.add_action(diagnostics_action)
        # QUADRA_PAGE opens another tab first (for screenshots).
        self.build(page or os.environ.get("QUADRA_PAGE", "play"))
        self._follow_settings()
        self._startup_checked = False  # the update check runs when settings first open
        threading.Thread(target=self._sync_ciderdarling_shortcut, daemon=True).start()
        threading.Thread(target=self._load_flag_allowlist, daemon=True).start()
        self._periodic_ciderdarling_updates()
        GLib.timeout_add_seconds(
            runtime_updates.CHECK_INTERVAL, self._periodic_ciderdarling_updates
        )

    # A browser join link waiting for the launcher expires like its file
    # (uri.PENDING_MAX_AGE): an old click must not start a game much later.
    @property
    def pending_uri(self):
        if (
            self._pending_uri is not None
            and time.monotonic() - self._pending_uri_at > uri_handoff.PENDING_MAX_AGE
        ):
            self._pending_uri = None
        return self._pending_uri

    @pending_uri.setter
    def pending_uri(self, value):
        if value != self._pending_uri:
            self._pending_uri_at = time.monotonic()
        self._pending_uri = value

    @property
    def session(self):
        return self._session

    @session.setter
    def session(self, value):
        if value is not self._session:
            self._session = value
            self.activity_changes.notify()

    @property
    def busy(self):
        return self._busy

    @busy.setter
    def busy(self, value):
        if value != self._busy:
            self._busy = value
            self.activity_changes.notify()

    def watch_setting(self, keys, callback):
        self._page_subscriptions.append(self.settings.watch(keys, callback))

    def watch_activity(self, callback):
        self._page_subscriptions.append(self.activity_changes.add(callback))

    def watch_client(self, callback):
        self._page_subscriptions.append(self.client_changes.add(callback))

    def watch_studio(self, callback):
        self._page_subscriptions.append(self.studio_status.watch(callback))

    def _stop_page_subscriptions(self):
        subscriptions, self._page_subscriptions = self._page_subscriptions, []
        for unsubscribe in subscriptions:
            unsubscribe()

    def client_changed(self):
        self.client_changes.notify()

    def _follow_settings(self):
        watch = self.settings.watch
        watch("runtime_backend", lambda _key, _value: self.apply_runtime_state())
        watch(
            "launcher_appearance",
            lambda _key, appearance: self._show_launcher_appearance(appearance),
        )
        watch("language", lambda _key, code: self._show_language(code))
        watch("discord_rpc", lambda _key, enabled: self._follow_discord_rpc(enabled))
        watch(
            ("discord_rpc_game", "discord_rpc_icon", "discord_rpc_time"),
            lambda _key, _value: self._refresh_rpc_presence(),
        )

    @staticmethod
    def _cider_pin_for(release):
        if runtime.is_bundled():
            return None
        return (
            runtime_updates.pin(release) if release else runtime.pinned_ciderdarling()
        )

    def _sync_ciderdarling_shortcut(self):
        """Keep the CiderDarling Settings menu entry in step with the runtimes."""
        try:
            installed = "ciderdarling" in runtime.installed_runtimes(
                core.DARLING_RUNTIME
            )
        except (OSError, RuntimeError, ValueError):
            return  # an unreadable selection: leave the entry as it is
        core.sync_ciderdarling_shortcut(installed)

    def show_ciderdarling_settings(self):
        if not self.settings_page.runtime_installed("ciderdarling") or os.environ.get(
            "FLATPAK_ID"
        ):
            self.stack.set_visible_child_name("settings")
            self.settings_page.set_tab("env")
            dialogs._toast(self.toasts, _("CiderDarling is not installed"))
            return
        self.stack.set_visible_child_name("settings")
        self.settings_page.set_tab("ciderdarling")

    def collect_diagnostics(self):
        """Write a diagnostics archive (diagnostics.py) without blocking GTK."""
        if self.diagnostics_running:
            dialogs._toast(self.toasts, _("Diagnostics are being collected…"))
            return
        try:
            from .. import diagnostics
        except ImportError as error:
            dialogs._error_dialog(self, _("Could not collect diagnostics"), str(error))
            return
        self.diagnostics_running = True
        toast = Adw.Toast.new(
            _("Collecting diagnostics… This can take up to a minute.")
        )
        toast.set_timeout(0)
        self.toasts.add_toast(toast)

        def done(archive, error):
            self.diagnostics_running = False
            toast.dismiss()
            if error:
                dialogs._error_dialog(
                    self, _("Could not collect diagnostics"), str(error) or repr(error)
                )
                return False
            self._diagnostics_ready_dialog(archive)
            return False

        def work():
            try:
                archive = diagnostics.collect()
                GLib.idle_add(done, archive, None)
            except Exception as error:  # shown to the user
                GLib.idle_add(done, None, error)

        threading.Thread(target=work, daemon=True).start()

    def _diagnostics_ready_dialog(self, archive):
        body = _(
            "Saved {name}. Attach it to your report; it leaves out sign-in data and join tickets.",
            name=archive.name,
        )
        if not self.settings.get("allow_debugger", False):
            body += "\n\n" + _(
                "Thread stacks need Allow freeze diagnostics (Settings → Environment → "
                "Diagnostics) to be on before the game starts."
            )
        dialog = Adw.AlertDialog(heading=_("Diagnostics collected"), body=body)
        dialog.add_response("close", _("Close"))
        dialog.add_response("open", _("Open folder"))
        dialog.set_response_appearance("open", Adw.ResponseAppearance.SUGGESTED)
        dialog.set_default_response("open")
        dialog.set_close_response("close")

        def response(_dialog, result):
            if result == "open":
                try:
                    Gio.AppInfo.launch_default_for_uri(archive.parent.as_uri(), None)
                except (OSError, GLib.Error) as error:
                    dialogs._toast(
                        self.toasts,
                        _("Could not open the folder: {error}", error=error),
                    )

        dialog.connect("response", response)
        dialog.present(self)

    # -- runtime backend (Darling or Rosemary)
    def runtime_name(self):
        return backends.ROSEMARY if self.rosemary_selected() else backends.DARLING

    def runtime_only(self, widget, feature, hide=False):
        """Register a settings row, group or page that only some runtimes read.
        It is greyed out, and tagged "<runtime> only", while another is chosen;
        with `hide`, it is hidden instead."""
        entry = {
            "widget": widget,
            "feature": feature,
            "hide": hide,
            "tag": None,
            "text": None,
        }
        label = " / ".join(backends.other_runtimes(feature))
        if not hide and isinstance(widget, Adw.PreferencesGroup):
            entry["text"] = widget.get_description() or ""
        elif not hide and hasattr(widget, "add_suffix"):
            tag = Gtk.Label(
                label=_("{runtime} only", runtime=label),
                css_classes=["dim-label", "caption"],
                valign=Gtk.Align.CENTER,
                visible=False,
            )
            widget.add_suffix(tag)
            entry["tag"] = tag
        self.runtime_widgets.append(entry)
        self._apply_runtime_entry(entry)

    def _apply_runtime_entry(self, entry):
        applies = backends.supports(self.runtime_name(), entry["feature"])
        widget = entry["widget"]
        if entry["hide"]:
            widget.set_visible(applies)
            return
        widget.set_sensitive(applies)
        label = " / ".join(backends.other_runtimes(entry["feature"]))
        if entry["tag"] is not None:
            entry["tag"].set_visible(not applies)
        elif entry["text"] is not None:
            note = "" if applies else _("{runtime} only.", runtime=label)
            widget.set_description((note + " " + entry["text"]).strip())

    def apply_runtime_state(self):
        for entry in self.runtime_widgets:
            self._apply_runtime_entry(entry)

    def rosemary_selected(self):
        return (
            self.settings.get(
                "runtime_backend", core.DEFAULT_SETTINGS["runtime_backend"]
            )
            == "rosemary"
        )

    def client_installed(self):
        """Whether there is a Roblox client to start with the chosen runtime."""
        if self.rosemary_selected():
            return rosemary_runtime.installed_version(self.settings) is not None
        return core.installed_version() is not None

    def rosemary_ready(self):
        return rosemary_runtime.rosemary_ready(self.settings)

    def runtime_ready(self):
        """False when starting needs the user first (no Darling client).
        Rosemary and its client are installed by Quadra when a start needs them."""
        if self.rosemary_selected():
            return True
        return bool(core.installed_version())

    # -- direct start: splash instead of the full window
    def present_settings(self):
        """Show the settings window (it stays hidden otherwise)."""
        self.settings_open = True
        if not self._startup_checked and not self.setup_active:
            self._startup_checked = True
            threading.Thread(target=self._check_startup_update, daemon=True).start()
        self.set_visible(True)
        self.present()

    def show_splash(self, text):
        if self.splash is None:
            self.splash = boot_splash.BootSplash(self)
            self.splash.set_application(self.get_application())
            self.splash.present()
        self.splash.set_status(text)

    def splash_update(self, text=None, fraction=None, indeterminate=False):
        if self.splash is None:
            return
        if text:
            self.splash.set_status(text)
        self.splash.set_progress(None if indeterminate else fraction)

    def close_splash(self):
        splash, self.splash = self.splash, None
        if splash is not None:
            splash.close_splash()

    def skip_preload(self):
        session = self._shader_preload_session
        if session is not None:
            session.shader_preload_cancel.set()

    def cancel_start(self):
        """Cancel on the splash: stop the game, or stop it as soon as it is up."""
        self.launch_cancelled = True
        self.close_splash()
        if self.session:
            self.stop()
        elif not self.busy:
            self._quit_if_quiet()

    def _quit_if_quiet(self):
        if not self.settings_open and not self.session and not self.busy:
            self.get_application().quit()

    def autostart(self):
        """Start Roblox right away (update first when one is out)."""
        self.launch_cancelled = False
        self.show_splash(_("Checking for updates…"))
        self.splash_update(indeterminate=True)
        threading.Thread(target=self._autostart_check, daemon=True).start()

    def _autostart_rosemary(self):
        """Install or update Roblox for Rosemary (the splash shows progress), then start."""

        def progress(fraction, text):
            GLib.idle_add(self.splash_update, text, fraction)

        try:
            if (
                self.settings.get("auto_check_roblox_updates", True)
                or not self.rosemary_ready()
            ):
                progress(0.0, _("Checking for updates…"))
                rosemary_runtime.ensure_ready(self.settings.snapshot(), progress)
        except Exception as error:
            GLib.idle_add(
                self._autostart_failed,
                _("Could not install Roblox for Rosemary"),
                str(error),
            )
            return
        GLib.idle_add(self._autostart_launch)

    def _autostart_failed(self, heading, details):
        self.close_splash()
        self.present_settings()
        dialogs._error_dialog(self, heading, details)
        return False

    def _autostart_check(self):
        if self.rosemary_selected():
            self._autostart_rosemary()
            return
        latest = upload = None
        if self.settings.get("auto_check_roblox_updates", True):
            try:
                installed = core.installed_version()
                channel = self.settings.get("roblox_channel", "") or ""
                if installed:
                    found, found_upload = core.latest_version(channel)
                    if core.version_is_newer(found, installed):
                        latest, upload = found, found_upload
            except Exception:
                pass  # offline: start what is installed
        GLib.idle_add(self._autostart_go, latest, upload)

    def _autostart_go(self, latest, upload):
        if self.launch_cancelled:
            self._quit_if_quiet()
            return False
        if latest:
            self.splash_update(_("Updating Roblox…"), 0.0)
            if self.splash:
                self.splash.cancel.set_sensitive(
                    False
                )  # a half unpacked client is worse than waiting
            started = self.settings_page.install_update(
                upload,
                expected_version=latest,
                on_progress=lambda fraction, text: self.splash_update(text, fraction),
                on_complete=self._autostart_updated,
            )
            if started:
                return False
        self._autostart_launch()
        return False

    def _autostart_updated(self, error):
        if error:
            self.close_splash()
            self.present_settings()
            dialogs._error_dialog(self, _("Update failed"), str(error) or repr(error))
            return
        if self.splash:
            self.splash.cancel.set_sensitive(True)
        self._autostart_launch()

    def _autostart_launch(self):
        if self.launch_cancelled:
            self._quit_if_quiet()
            return
        self.splash_update(_("Starting Roblox…"), indeterminate=True)
        self.play_clicked()
        if not self.busy and not self.session:
            # Nothing started (setup is needed, or another operation runs).
            self.close_splash()
            self.present_settings()

    def notify_server_location(self, location, ip=None):
        """Tell the user where the server they joined is (desktop notification;
        the launcher is usually hidden while a game runs). Once per server."""
        if not self.settings.get("server_location_notify", True):
            return
        if ip is not None:
            if ip == self._notified_server_ip:
                return
            self._notified_server_ip = ip
        notification = Gio.Notification.new(_("Server joined"))
        notification.set_body(
            _("Your server is located in {location}").format(location=location)
        )
        notification.set_icon(Gio.ThemedIcon.new("quadrahq"))
        # One id: a new server replaces the previous notification.
        self.get_application().send_notification("server-location", notification)

    def build(self, page):
        """(Re)create the interface, e.g. after the language changes."""
        if self.settings_page:
            self.settings_page.flush()  # the new page reads the file
        if self.studio_page:
            self.studio_page.flush()
        if self.play_page:
            self.play_page.logs_view.stop_server_location_polling()
        self._stop_page_subscriptions()
        self.toasts = Adw.ToastOverlay()
        self.runtime_widgets = []
        self.stack = Adw.ViewStack()
        self.play_page = play_page.PlayPage(self)
        self.stack.add_titled_with_icon(
            self.play_page, "play", _("Play"), "media-playback-start-symbolic"
        )
        self.settings_page = settings_page.SettingsPage(self)
        self.stack.add_titled_with_icon(
            self.settings_page, "settings", _("Settings"), "emblem-system-symbolic"
        )
        # Studio runs through Wine, which the Flatpak does not have.
        if not os.path.exists("/.flatpak-info"):
            self.studio_page = studio_page.StudioPage(self)
            self.stack.add_titled_with_icon(
                self.studio_page,
                "studio",
                _("Studio"),
                "applications-development-symbolic",
            )
        self.servers_page = servers_page.ServersPage(self)
        self.stack.add_titled_with_icon(
            self.servers_page, "servers", _("Servers"), "network-server-symbolic"
        )
        self.mods_page = mods_page.ModsPage(self)
        self.stack.add_titled_with_icon(
            self.mods_page, "mods", _("Mods"), "application-x-addon-symbolic"
        )
        self.info_page = info_page.InfoPage(self)
        self.stack.add_titled_with_icon(
            self.info_page, "info", _("Info"), "help-about-symbolic"
        )
        self.flags_page = self.settings_page.flags_page

        if page == "ciderdarling":
            self.show_ciderdarling_settings()
        elif page in ("flags", "env", "m2vk", "roblox", "vr"):
            self.stack.set_visible_child_name("settings")
            self.settings_page.set_tab(page)
        elif self.stack.get_child_by_name(page) is not None:
            self.stack.set_visible_child_name(page)

        # Official Libadwaita Split View layout
        self.split = Adw.OverlaySplitView()
        self.split.set_min_sidebar_width(200)
        self.split.set_max_sidebar_width(260)
        self.split.set_sidebar_width_fraction(0.28)
        self.split.set_show_sidebar(self.settings.get("show_sidebar", True))

        # Sidebar with the page list
        sidebar_toolbar = Adw.ToolbarView()
        self.sidebar_toolbar = sidebar_toolbar
        sidebar_header = Adw.HeaderBar(
            show_end_title_buttons=False, show_start_title_buttons=False
        )
        self.brand_label = Gtk.Label(label="Quadra", css_classes=["heading"])
        sidebar_header.set_title_widget(self.brand_label)
        sidebar_version = Gtk.Label(
            label=f"v{__version__}", css_classes=["dim-label", "caption"], margin_end=6
        )
        sidebar_header.pack_end(sidebar_version)
        sidebar_toolbar.add_top_bar(sidebar_header)

        sidebar_toolbar.set_content(widgets._page_sidebar(self.stack))
        self.split.set_sidebar(sidebar_toolbar)

        # Content area
        content_view = Adw.ToolbarView()
        header = Adw.HeaderBar()
        self.launcher_header = header
        menu = Gio.Menu()
        menu.append(_("Setup guide"), "win.setup")
        menu.append(_("Collect diagnostics"), "win.collect-diagnostics")
        header.pack_end(
            Gtk.MenuButton(
                icon_name="open-menu-symbolic",
                menu_model=menu,
                tooltip_text=_("Launcher menu"),
            )
        )

        sidebar_toggle = Gtk.Button(icon_name="sidebar-show-symbolic")
        sidebar_toggle.add_css_class("flat")
        sidebar_toggle.set_tooltip_text(_("Toggle sidebar"))
        sidebar_toggle.connect("clicked", lambda *_args: self.toggle_sidebar())
        header.pack_start(sidebar_toggle)

        content_view.add_top_bar(header)
        self.toasts.set_child(self.stack)
        content_view.set_content(self.toasts)
        content_view.set_hexpand(True)
        content_view.set_vexpand(True)
        self.split.set_content(content_view)

        self.setup = SetupWizard(self)
        self.content_stack = Gtk.Stack(
            transition_type=Gtk.StackTransitionType.CROSSFADE, transition_duration=150
        )
        responsive_split = Adw.BreakpointBin(
            child=self.split, width_request=360, height_request=320
        )
        compact = Adw.Breakpoint.new(Adw.BreakpointCondition.parse("max-width: 760sp"))
        compact.add_setter(self.split, "collapsed", True)
        compact.add_setter(self.split, "show-sidebar", False)
        responsive_split.add_breakpoint(compact)
        self.content_stack.add_named(responsive_split, "launcher")
        self.content_stack.add_named(self.setup, "setup")
        self.backdrops = Gtk.Stack()
        self.backdrops.add_named(theme.GridBackdrop(), "grid")
        self.backdrops.add_named(theme.Backdrop(), "monochrome")
        self.backdrops.add_named(Gtk.Box(), "gnome")
        self.wallpaper = Gtk.Overlay(child=self.backdrops)
        self.wallpaper.add_overlay(self.content_stack)
        self.wallpaper.set_measure_overlay(self.content_stack, True)
        self.appearance_layer = theme.AppearanceLayer(self.wallpaper)
        self.set_content(self.appearance_layer)
        self.apply_launcher_appearance()
        if page in ("setup", "continue-setup") or (
            page not in ("settings", "ciderdarling", "env", "roblox", "flags")
            and not self.client_installed()
            and not self.settings.get("setup_complete", False)
        ):
            self.show_setup()
            if page == "continue-setup":
                GLib.idle_add(self.setup.continue_install)
        else:
            self.setup_active = False
            self.content_stack.set_visible_child_name("launcher")
        if self.session is not None and self.session.log_path:
            self.play_page.logs_view.reset(self.session.log_path)

    def apply_launcher_appearance(self):
        appearance = self.settings.get("launcher_appearance", "gnome")
        custom = appearance != "gnome"
        self.backdrops.set_visible_child_name(appearance)
        self.appearance_layer.set_appearance(appearance)
        self.settings_page.appearance_tabs.set_visible_child_name(
            "custom" if custom else "gnome"
        )
        if self.studio_page:
            self.studio_page.appearance_tabs.set_visible_child_name(
                "custom" if custom else "gnome"
            )
        for widget, css in (
            (self, "launcher-custom"),
            (self.content_stack, "launcher-shell"),
            (self.sidebar_toolbar, "launcher-sidebar"),
            (self.brand_label, "launcher-brand"),
            (self.launcher_header, "launcher-header"),
        ):
            (widget.add_css_class if custom else widget.remove_css_class)(css)
        self.play_page.set_appearance(appearance)

    def set_launcher_appearance(self, appearance):
        if appearance in ("grid", "gnome", "monochrome"):
            self.set_setting("launcher_appearance", appearance)

    def _show_launcher_appearance(self, appearance):
        theme.install(appearance)
        self.apply_launcher_appearance()

    def show_setup(self, page="welcome"):
        if self.session or self.busy:
            dialogs._toast(
                self.toasts,
                _("Close Roblox first")
                if self.session
                else _("Please wait, the launcher is busy"),
            )
            return
        self.setup_active = True
        self.content_stack.set_visible_child_name("setup")
        self.setup.show(page)

    def dismiss_setup(self):
        if self.setup.installing:
            return
        self.setup_active = False
        self.content_stack.set_visible_child_name("launcher")

    def complete_setup(self, launch=False):
        self.set_setting("setup_complete", True)
        self.dismiss_setup()
        self.stack.set_visible_child_name("play")
        if launch:
            self.play_clicked()

    def toggle_sidebar(self):
        show = not self.split.get_show_sidebar()
        self.split.set_show_sidebar(show)
        self.set_setting("show_sidebar", show)

    def _check_startup_update(self):
        try:
            has_update, tag, html_url = core.check_launcher_update()
            if has_update:
                GLib.idle_add(self._show_launcher_update_dialog, tag)
                return
        except Exception:
            pass

        if not self.rosemary_selected() and self.settings.get(
            "auto_check_roblox_updates", True
        ):
            try:
                installed = core.installed_version()
                if installed:
                    channel = ""
                    latest, upload = core.latest_version(channel)
                    if core.version_is_newer(latest, installed):
                        GLib.idle_add(
                            self._show_roblox_update_dialog, latest, upload, channel
                        )
            except Exception:
                pass

    def _show_launcher_update_dialog(self, tag):
        dialog = Adw.AlertDialog(
            heading=_("Update available"),
            body=_(
                "A new version of Quadra ({version}) is available. Update now?",
                version=tag,
            ),
        )
        dialog.add_response("later", _("Later"))
        dialog.add_response("update", _("Update"))
        dialog.set_response_appearance("update", Adw.ResponseAppearance.SUGGESTED)

        def response(_dialog, result):
            if result == "update":
                self.stack.set_visible_child_name("settings")
                self.settings_page.set_tab("env")
                self.settings_page.force_update_launcher()

        dialog.connect("response", response)
        dialog.present(self)

    def _show_roblox_update_dialog(self, latest, upload, channel=None):
        dialog = Adw.AlertDialog(
            heading=_("Roblox update available"),
            body=_(
                "A newer version of Roblox ({version}) is available. Update now?",
                version=latest,
            ),
        )
        dialog.add_response("later", _("Later"))
        dialog.add_response("update", _("Update"))
        dialog.set_response_appearance("update", Adw.ResponseAppearance.SUGGESTED)

        def response(_dialog, result):
            if result == "update":
                self.stack.set_visible_child_name("settings")
                self.settings_page.set_tab("roblox")
                self.settings_page.install_update(
                    upload, expected_version=latest, channel=channel
                )

        dialog.connect("response", response)
        dialog.present(self)

    def begin(self, what):
        """Claim the launcher for one long operation; False, with a message,
        while the game runs or another operation is under way."""
        if self.session or self.busy:
            dialogs._toast(
                self.toasts,
                _("Close Roblox first")
                if self.session
                else _("Please wait, the launcher is busy"),
            )
            return False
        self.busy = what
        return True

    def end(self, resume_pending=True):
        self.busy = None
        if self.quit_when_idle:
            self.get_application().quit()
            return
        if (
            resume_pending
            and self.pending_uri
            and self.runtime_ready()
            and not self.setup_active
        ):
            pending = self.pending_uri
            GLib.idle_add(self.handle_uri, pending)

    def set_setting(self, key, value):
        self.settings[key] = value

    def _periodic_ciderdarling_updates(self):
        if self.settings.get("auto_check_ciderdarling_updates", True):
            self.check_ciderdarling_updates()
        return True

    def check_ciderdarling_updates(self, force=False):
        if runtime.is_bundled() or self.cider_update_checking:
            return
        self.cider_update_checking = True
        self.settings_page.refresh_ciderdarling()

        def worker():
            release, error = None, None
            try:
                release = runtime_updates.check(core.CACHE_DIR, force=force)
            except (OSError, ValueError, TypeError) as exception:
                error = str(exception)
            GLib.idle_add(self._runtime_update_checked, release, error, force)

        threading.Thread(target=worker, daemon=True).start()

    def _runtime_update_checked(self, release, error, force):
        self.cider_update_checking = False
        page = self.settings_page
        # A running install uses the exact release the user accepted. A
        # subsequent check can present a newer release after it finishes.
        if release is not None and self.runtime_install_job is None:
            self.cider_release = release
            self.cider_pin = runtime_updates.pin(release)
            page.refresh_runtime_choices()
        page.refresh_ciderdarling()
        if page.ciderdarling_update_needed():
            revision = self.cider_release["source_revision"]
            if force or revision != self._notified_cider_revision:
                self._notified_cider_revision = revision
                dialogs._toast(
                    self.toasts,
                    _("A CiderDarling update is available in Settings → CiderDarling."),
                )
        elif force:
            dialogs._toast(
                self.toasts,
                _("Could not check CiderDarling updates: {error}", error=error)
                if error
                else _("No CiderDarling update is available."),
            )
        return False

    def install_ciderdarling(self):
        """Install CiderDarling, or update it to the build Quadra's installer
        pins, in a separate terminal (runtime_install.TERMINAL_SCRIPT)."""
        if runtime.is_bundled():
            return
        page = self.settings_page
        update = page.ciderdarling_update_needed()
        if not update and page.selected_runtime() != "ciderdarling":
            return
        if not self.begin("installing runtime"):
            return
        job = None
        try:
            if core.roblox_pids():
                self.end(resume_pending=False)
                dialogs._toast(self.toasts, _("Close Roblox first"))
                return
            if not update and "ciderdarling" in runtime.installed_runtimes(
                core.DARLING_RUNTIME
            ):
                self.end(resume_pending=False)
                page.refresh_runtime_choices()
                return
            job = runtime_install.Installation.create(
                core.PROJECT, "ciderdarling", core.CACHE_DIR, release=self.cider_release
            )
            if not system._launch_runtime_terminal(self, job.command):
                raise RuntimeError(_("Could not open an installation terminal."))
            self.runtime_install_job = job
            page._runtime_selection_changed()
            page.refresh_ciderdarling()
            GLib.timeout_add(1000, self._watch_runtime_install, job)
        except (GLib.Error, OSError, RuntimeError, ValueError) as error:
            if job is not None:
                try:
                    job.close()
                except OSError:
                    pass
            self.end(resume_pending=False)
            dialogs._error_dialog(self, _("Could not install CiderDarling"), str(error))

    def _watch_runtime_install(self, job):
        if self.runtime_install_job is not job:
            return False
        error = None
        try:
            status = job.poll()
            if status is None:
                return True
            if status != 0:
                error = _(
                    "Installation failed or was cancelled. See the separate terminal for details."
                )
                tail = job.log_tail(40)
                if tail:
                    error += "\n\n" + _("End of the build log:") + "\n" + tail
        except (OSError, RuntimeError, ValueError) as exception:
            error = str(exception)
        self.runtime_install_job = None
        try:
            job.close()
        except OSError as exception:
            error = error or str(exception)
        self.end(resume_pending=False)
        page = self.settings_page
        page.refresh_runtime_choices()
        choice = page.installed_runtime("ciderdarling")
        if error:
            dialogs._error_dialog(self, _("Could not install CiderDarling"), error)
        elif choice is None or (
            job.target_revision is not None
            and runtime.metadata(choice["sysroot"]).get("source_revision")
            != job.target_revision
        ):
            dialogs._error_dialog(
                self,
                _("Could not detect CiderDarling"),
                _(
                    "The installer finished, but the runtime could not be detected. Check the terminal output."
                ),
            )
        else:
            core.clear_last_error()
            self._remember_installed_ciderdarling(choice)
            threading.Thread(
                target=self._sync_ciderdarling_shortcut, daemon=True
            ).start()
            page.select_runtime("ciderdarling")
            if (
                core.DARLING_RUNTIME["runtime"] == "ciderdarling"
                and page.runtime_apply_button.get_sensitive()
            ):
                dialogs._toast(
                    self.toasts,
                    _("CiderDarling updated. Apply and restart Quadra to use it."),
                )
            else:
                dialogs._toast(
                    self.toasts,
                    _("CiderDarling installed. Apply the runtime choice to switch."),
                )
        return False

    def _remember_installed_ciderdarling(self, choice):
        """A saved CiderDarling (active or remembered) moves to the new
        installation, keeping its prefix; another active runtime stays."""
        try:
            saved = runtime.read_selection(runtime.selection_path())
            remembered = runtime.remembered_runtimes(saved).get("ciderdarling")
            if remembered and any(
                remembered[key] != choice[key] for key in runtime.SELECTION_KEYS
            ):
                runtime.remember_runtime(choice)
        except (OSError, RuntimeError, ValueError) as error:
            print("Could not remember the installed CiderDarling:", error)

    def collect_hardware_report(self):
        if not self.begin("collecting hardware report"):
            return

        def worker():
            path, error = None, None
            try:
                if core.roblox_pids():
                    raise RuntimeError(_("Close Roblox first"))
                from .. import hardware_report

                path = hardware_report.collect(graphics=True)
            except (OSError, RuntimeError, ValueError) as exception:
                error = str(exception)
            GLib.idle_add(done, path, error)

        def done(path, error):
            self.end(resume_pending=False)
            if error:
                dialogs._error_dialog(
                    self, _("Could not collect hardware report"), error
                )
            else:
                dialog = Adw.AlertDialog(
                    heading=_("Hardware report saved"), body=str(path)
                )
                dialog.add_response("ok", _("OK"))
                dialog.present(self)
            return False

        threading.Thread(target=worker, daemon=True).start()

    def set_language(self, code):
        self.set_setting("language", code)

    def _show_language(self, code):
        if code == i18n.language():
            return
        i18n.set_language(code)
        # Rebuild after the combo row finished handling its own signal.
        GLib.idle_add(lambda: self.build("settings") and False)

    def _captcha_dialog(self):
        dialog = Adw.AlertDialog(
            heading=_("Roblox closed at the captcha"),
            body=_(
                "Signing up and signing in with a password show a captcha in a built-in browser. "
                "The launcher shows it in a window of its own when WebKitGTK 6.0 is installed "
                "(webkitgtk-6.0, gir1.2-webkit-6.0 or webkitgtk6.0). Without it, create the "
                "account on roblox.com, then sign in with Quick Login: Roblox shows a code, "
                "enter it on a phone or in a browser where you are already signed in."
            ),
        )
        dialog.add_response("ok", _("OK"))
        dialog.present(self)

    def play_clicked(self):
        if self.rosemary_selected() or core.installed_version():
            if self.pending_uri:
                self.handle_uri(self.pending_uri)
            else:
                self.launch()
        else:
            self.show_setup("overview")

    def handle_uri(self, browser_uri):
        """Start Roblox with an opaque browser handoff when it is ready."""
        # The activation callback may have been queued just before another
        # browser click arrived.  Prefer the still-pending complete argument
        # so the newest handoff is the one that starts the client.
        browser_uri = uri_handoff.peek_pending() or browser_uri
        self.pending_uri = browser_uri
        if (
            self.session
            or self.busy
            or self.setup_active
            or self.stopping
            or self._update_required_dialog_active
        ):
            # The pending file remains in place.  A later activation or the
            # end of the current operation will retry this exact argument.
            return
        if not self.rosemary_selected() and not core.installed_version():
            self.show_setup("overview")
            return
        # Keep the file until RobloxSession has started successfully.  If
        # Darling or the shim fails, the browser handoff can still be retried.
        self.launch(browser_uri)

    def studio_clicked(self):
        if self.studio_status.value is not None:
            return
        if studio.running():
            dialogs._toast(self.toasts, _("Roblox Studio is already running"))
            return
        if not studio.needs_install():
            self._update_and_start_studio()
            return
        dialog = Adw.AlertDialog(
            heading=_("Install Roblox Studio?"),
            body=_(
                "Studio runs in its Windows version through Wine. Quadra downloads Wine, "
                "DXVK, Microsoft's WebView2 and Studio, about 800 MB."
            ),
        )
        dialog.add_response("cancel", _("Cancel"))
        dialog.add_response("install", _("Install"))
        dialog.set_response_appearance("install", Adw.ResponseAppearance.SUGGESTED)
        dialog.connect(
            "response",
            lambda _d, result: result == "install" and self._update_and_start_studio(),
        )
        dialog.present(self)

    def _update_and_start_studio(self):
        """Installs or updates Studio when needed, then starts it."""
        if self.studio_status.value is not None:
            return
        if self.studio_page:
            self.studio_page.flush()  # Studio reads the flags when it starts
        self.studio_status.set((None, ""))

        def progress(fraction, text):
            GLib.idle_add(self.studio_status.set, (fraction, text))

        def work():
            settings = self.settings.snapshot()
            try:
                studio.install(progress, settings)
            except studio.ChannelError:
                raise
            except Exception as error:
                if studio.needs_install():
                    raise
                # Offline or Roblox unreachable: start the installed version.
                print("Studio update skipped:", error)
            # A process of its own looks after Studio (GameMode, Discord, the sign-in backup).
            studio.spawn()

        def done(_result, error):
            self.studio_status.set(None)
            if self.studio_page:
                self.studio_page.refresh()
            if error:
                dialogs._error_dialog(
                    self, _("Could not start Roblox Studio"), str(error) or repr(error)
                )
            else:
                dialogs._toast(self.toasts, _("Starting Roblox Studio…"))

        def run():
            try:
                work()
                GLib.idle_add(done, None, None)
            except Exception as error:
                GLib.idle_add(done, None, error)

        threading.Thread(target=run, daemon=True).start()

    def _rosemary_install_then_launch(self, launch_uri):
        """Rosemary or its Roblox client is missing: install them, with the splash
        showing progress, then start with the same link."""
        if not self.begin("updating"):
            return
        self.show_splash(_("Installing Rosemary and Roblox…"))
        self.splash_update(indeterminate=True)
        if self.splash:
            self.splash.cancel.set_sensitive(
                False
            )  # a half unpacked install is worse than waiting

        def progress(fraction, text):
            GLib.idle_add(self.splash_update, text or None, fraction)

        settings = self.settings.snapshot()

        def work():
            error = None
            try:
                rosemary_runtime.ensure_ready(settings, progress)
            except Exception as failure:
                error = failure
            GLib.idle_add(finished, error)

        def finished(error):
            self.close_splash()
            self.end(resume_pending=False)
            self.client_changed()
            if error is not None:
                self.present_settings()
                dialogs._error_dialog(
                    self,
                    _("Could not install Roblox with Rosemary"),
                    str(error) or repr(error),
                )
                return False
            self.launch(
                launch_uri, _installed=True
            )  # a second miss is an error, not another install
            return False

        threading.Thread(target=work, daemon=True).start()

    def launch(self, launch_uri=None, _installed=False):
        if (
            self.rosemary_selected()
            and not _installed
            and not self.session
            and not self.busy
            and not self.rosemary_ready()
        ):
            self._rosemary_install_then_launch(launch_uri)
            return
        if self.stopping or not self.begin("starting"):
            return
        self.settings_page.flush()
        if self.rosemary_selected():
            session = rosemary_runtime.RosemarySession(
                self.settings.snapshot(), launch_uri=launch_uri
            )
        else:
            session = core.RobloxSession(
                self.settings.snapshot(), launch_uri=launch_uri
            )
        self._shader_preload_session = session
        session.shader_preload_progress = lambda done, total: GLib.idle_add(
            self._show_shader_preload, session, done, total
        )
        session.preload_stage_progress = lambda phase, done, total: GLib.idle_add(
            self._show_shader_preload, session, done, total, phase
        )
        self.web = None if self.rosemary_selected() else self._web_bridge()
        if self.web:
            session.web_socket = self.web.guest_path
            session.web_user_agent = self.web.user_agent

        def start():
            try:
                session.start()  # cleans up after itself when it fails
                GLib.idle_add(self._close_shader_preload, session)
                GLib.idle_add(self._started, session, None)
            except Exception as error:
                GLib.idle_add(self._close_shader_preload, session)
                GLib.idle_add(self._started, session, error)

        threading.Thread(target=start, daemon=True).start()

    def _show_shader_preload(self, session, done, total, phase="pipelines"):
        if (
            self._shader_preload_session is not session
            or session.shader_preload_cancel.is_set()
            or self.busy != "starting"
        ):
            return False
        if self.splash is not None:
            labels = {
                "libraries": _("Preparing shader libraries"),
                "pipelines": _("Warming learned shaders"),
                "resources": _("Warming local textures and fonts"),
            }
            self.splash.skip.set_visible(True)
            self.splash_update(
                labels.get(phase, _("Preparing Roblox")), done / total if total else 0
            )
            return False
        dialog = self._shader_preload_dialog
        if dialog is None:
            dialog = Adw.AlertDialog(
                heading=_("Preparing Roblox"),
                body=_(
                    "Preparing shaders and local resources. You can skip and start playing."
                ),
            )
            self._shader_preload_bar = Gtk.ProgressBar(show_text=True)
            dialog.set_extra_child(self._shader_preload_bar)
            dialog.add_response("skip", _("Skip and play"))
            dialog.set_close_response("skip")
            self._shader_preload_response = dialog.connect(
                "response", lambda *_args: session.shader_preload_cancel.set()
            )
            self._shader_preload_dialog = dialog
            dialog.present(self)
        labels = {
            "libraries": _("Preparing shader libraries"),
            "pipelines": _("Warming learned shaders"),
            "resources": _("Warming local textures and fonts"),
        }
        dialog.set_heading(labels.get(phase, _("Preparing Roblox")))
        self._shader_preload_bar.set_fraction(done / total if total else 0)
        self._shader_preload_bar.set_text(
            _("{done} of {total} items", done=done, total=total)
        )
        return False

    def _close_shader_preload(self, session):
        if self._shader_preload_session is session:
            dialog = self._shader_preload_dialog
            if dialog is not None:
                dialog.disconnect(self._shader_preload_response)
                dialog.close()
            self._shader_preload_dialog = None
            self._shader_preload_session = None
        return False

    def _web_bridge(self):
        """The browser window for Roblox's embedded pages (sign-in with a
        password, purchases), or None when WebKitGTK is not installed."""
        try:
            from ..darling import web

            return web.WebBridge(self)
        except Exception as error:  # a missing WebKit, a socket that cannot be made
            print("Embedded web pages are not available:", error)
            return None

    def _stop_web(self):
        if self.web:
            self.web.stop()
            self.web = None

    def _follow_discord_rpc(self, enabled):
        if not enabled:
            self._stop_rpc()
        elif self.session:
            self._start_rpc()

    def _on_game_activity_change(self, info: dict | None, session, tracker, generation):
        # The log and metadata workers only enqueue; GTK-owned state changes
        # after checking that this callback still belongs to the active launch.
        GLib.idle_add(
            self._apply_game_activity_change, info, session, tracker, generation
        )

    def _apply_game_activity_change(self, info, session, tracker, generation):
        if (
            self.session is not session
            or self.game_tracker is not tracker
            or not self.settings.get("discord_rpc", True)
            or not tracker.is_current_generation(generation)
        ):
            return False
        self.current_game_info = info
        self._refresh_rpc_presence()
        return False

    def _refresh_rpc_presence(self):
        if not self.session or not self.settings.get("discord_rpc", True):
            return False
        if self.rpc is None or self.rpc.closed:
            try:
                self.rpc = discord.DiscordRPC()
            except RuntimeError as error:
                print("Could not start Discord presence worker:", error)
                return False
        rpc = self.rpc
        start = (
            self.game_started_at or time.time()
            if self.settings.get("discord_rpc_time", True)
            else None
        )
        info = self.current_game_info
        brand = f"Quadra {__version__}"
        large_image, large_text = discord.LOGO, brand
        small_image = small_text = None
        buttons = [(_("Get Quadra"), discord.WEBSITE_URL)]

        if (
            info
            and not info.get("loading")
            and self.settings.get("discord_rpc_game", True)
        ):
            details = info.get("name", _("Playing Roblox"))
            creator = info.get("creator")
            state = _("by {creator}", creator=creator) if creator else _("In Game")
            icon_url = info.get("icon_url")
            if self.settings.get("discord_rpc_icon", False) and icon_url:
                large_image, large_text = icon_url, details
                small_image, small_text = discord.LOGO, brand
            if info.get("place_id"):
                buttons.insert(
                    0,
                    (
                        _("Game page"),
                        f"https://www.roblox.com/games/{info['place_id']}",
                    ),
                )
        elif info:
            # Game is active (either fetching details or user hid experience name in settings)
            details = _("Playing Roblox")
            state = _("In Game")
        else:
            # Menu (not in an experience)
            details = _("In Main Menu")
            state = None

        rpc.queue_presence(
            details=details,
            state=state,
            start_time=start,
            large_image=large_image,
            large_text=large_text,
            small_image=small_image,
            small_text=small_text,
            buttons=buttons,
        )
        return False

    def _start_rpc(self):
        session = self.session
        if not session or not self.settings.get("discord_rpc", True):
            return
        if self.game_tracker is None and session.log_path:
            tracker = discord.GameActivityTracker(
                session.game_log if hasattr(session, "game_log") else session.log_path,
                lambda info, generation: self._on_game_activity_change(
                    info, session, tracker, generation
                ),
                autostart=False,
            )
            self.game_tracker = tracker
            try:
                tracker.start()
            except RuntimeError as error:
                self.game_tracker = None
                print("Could not start Discord game tracker:", error)
        self._refresh_rpc_presence()

    def _stop_rpc(self):
        tracker, self.game_tracker = self.game_tracker, None
        if tracker:
            tracker.stop()
        self.current_game_info = None
        if self.rpc:
            rpc = self.rpc
            self.rpc = None
            rpc.close()

    def _started(self, session, error):
        self.busy = None
        self.quit_when_idle = False  # a game or an error to show: stay
        if error:
            self.close_splash()
            self._stop_web()
            if session and session.launch_uri:
                self.pending_uri = session.launch_uri
            self.present_settings()
            dialogs._error_dialog(
                self, _("Could not start Roblox"), str(error) or repr(error)
            )
            return
        self._stop_rpc()
        self.session = session
        if self.launch_cancelled:
            # Cancelled while it was starting: end it as soon as it is up.
            self.stop()
        else:
            self.splash_update(_("Roblox is starting…"), indeterminate=True)
        reason = getattr(session, "metal_fallback_reason", None)
        if reason:
            dialogs._toast(
                self.toasts,
                _(
                    "Quadra M2VK could not start; playing with OpenGL this time. {reason}",
                    reason=reason.splitlines()[0][:160],
                ),
            )
        if self._transport_warning_dialog is not None:
            self._transport_warning_dialog.close()
        self._transport_warning_session = None
        core.clear_last_error()
        if session.launch_uri:
            # A newer click may have replaced pending-uri while Darling was
            # starting; clear only the argument this session consumed.
            uri_handoff.clear_pending(session.launch_uri)
            self.pending_uri = uri_handoff.peek_pending()
        self.last_log = session.log_path
        self.game_started_at = time.time()
        self._playtime_clock = time.monotonic()
        self._playtime_saved_at = self._playtime_clock
        self._playtime_before_session = int(self.settings.get("playtime_seconds", 0))
        self._rpc_retry_at = self._playtime_clock + 5
        self.current_game_info = None
        self._notified_server_ip = None
        self._start_activity(session)
        if int(self.settings.get("roblox_cpu_limit", 0) or 0) > 0:
            GLib.timeout_add_seconds(10, self._limit_cpus, session)
        self.play_page.logs_view.reset(session.log_path)
        self.play_page.logs_view.update()
        if self.settings.get("discord_rpc", True):
            self._start_rpc()
        # Hide once the game window has had time to appear.
        GLib.timeout_add_seconds(3, self._hide_while_playing)
        GLib.timeout_add(1000, self._watch, session)

    def _hide_while_playing(self):
        self.close_splash()
        if (
            self.session
            and self.settings.get("hide_launcher_on_launch", True)
            and self._transport_warning_dialog is None
        ):
            self.set_visible(False)
        return False

    def _show_transport_warning(self, session):
        if (
            self.session is not session
            or self._transport_warning_session is session
            or not session.transport_monitor.warning_transport
        ):
            return
        self._transport_warning_session = session
        dialog = Adw.AlertDialog(
            heading=_("Roblox update broke game compatibility"),
            body=_(
                "Roblox is no longer using RakNet. The latest Roblox update has broken "
                "game connection compatibility with Quadra. Games might not work or may "
                "show Error 256.\n\nCheck for a Quadra update. If a fix is not available yet, "
                "wait for a Quadra compatibility patch."
            ),
        )
        self._transport_warning_dialog = dialog
        dialog.add_response("close", _("Close"))
        dialog.add_response("logs", _("View log"))
        dialog.add_response("updates", _("Check for updates"))
        dialog.set_default_response("close")
        dialog.set_close_response("close")

        def response(_dialog, choice):
            if self._transport_warning_dialog is dialog:
                self._transport_warning_dialog = None
            if self._transport_warning_session is not session or self.session not in (
                session,
                None,
            ):
                return
            if choice == "logs":
                self.play_page.show_logs(session.log_path)
            elif choice == "updates":
                self.stack.set_visible_child_name("settings")
                self.settings_page.set_tab("env")
                self.settings_page.check_launcher_update()

        dialog.connect("response", response)
        self.set_visible(True)
        self.present()
        dialog.present(self)

    def _watch(self, session):
        # A timer belongs to one launch. It must never start polling a new
        # session if the previous one finished between timer turns.
        if self.session is not session:
            return False
        if self._watch_inflight is not None:
            return True
        self._watch_inflight = session

        def poll():
            try:
                status = session.poll()
            except Exception as error:
                print("Watching the game failed:", error)
                try:
                    session.finish()
                    status = -1
                except Exception as cleanup_error:
                    # Keep ownership of this session until cleanup succeeds;
                    # the next timer turn retries without overlapping launches.
                    print("Cleaning up the game failed:", cleanup_error)
                    status = None
            GLib.idle_add(self._watch_result, session, status)

        try:
            threading.Thread(target=poll, daemon=True).start()
        except RuntimeError as error:
            self._watch_inflight = None
            print("Could not start game watcher:", error)
        return True

    def _load_flag_allowlist(self):
        fflag_allowlist.load_cached(core.CACHE_DIR)
        GLib.idle_add(self._allowlist_loaded)
        fflag_allowlist.refresh(core.CACHE_DIR)
        GLib.idle_add(self._allowlist_loaded)

    def _allowlist_loaded(self):
        for page in (
            getattr(self.settings_page, "flags_page", None),
            getattr(self, "studio_page", None),
        ):
            page = getattr(page, "flags_page", page)
            if page is not None and hasattr(page, "refresh_allowlist"):
                page.refresh_allowlist()
        return False

    def _rosemary_update_and_relaunch(self):
        """The client closed to update: install the current build (dropping a
        pinned older one), then start again with the same link."""
        if not self.begin("updating"):
            return
        self.show_splash(_("Updating Roblox…"))
        self.splash_update(indeterminate=True)
        pending = self.pending_uri
        settings = self.settings.snapshot()

        def progress(fraction, text):
            GLib.idle_add(self.splash_update, text, fraction)

        def work():
            error = None
            try:
                rosemary_runtime.ensure_latest(settings, progress, ignore_pin=True)
            except Exception as failure:
                error = failure
            GLib.idle_add(finished, error)

        def finished(error):
            self.close_splash()
            self.end(resume_pending=False)
            self.client_changed()
            if error is not None:
                self.present_settings()
                dialogs._error_dialog(
                    self, _("Could not update Roblox"), str(error) or repr(error)
                )
                return False
            if self.settings.get("rosemary_pinned_build"):
                self.set_setting("rosemary_pinned_build", "")
            self.launch(pending)
            return False

        threading.Thread(target=work, daemon=True).start()

    def _limit_cpus(self, session):
        if self.session is session and not getattr(session, "cpu_limited", False):
            threading.Thread(
                target=core.limit_roblox_cpus,
                args=(int(self.settings.get("roblox_cpu_limit", 0)),),
                daemon=True,
            ).start()
        return False

    def _start_activity(self, session):
        """Follow the player log for joins: server history, and the server
        location notification as soon as the server is known."""
        self._stop_activity()
        self._activity_join = None
        if not session.log_path:
            return
        target = session.game_log if hasattr(session, "game_log") else session.log_path
        self.activity = activity.ActivityWatcher(target, self._on_log_event).start()

    def _stop_activity(self):
        watcher = self.activity
        if watcher is not None:
            watcher.stop()
        self.activity = None

    def _on_log_event(self, event):
        """Runs on the watcher thread: no GTK calls here."""
        if event.name == logevents.JOINING:
            self._activity_join = (
                event.data["place_id"],
                event.data["job_id"],
                event.data["server_ip"],
            )
            history.record(
                core.CONFIG_DIR,
                event.data["place_id"],
                event.data["job_id"],
                server_ip=event.data["server_ip"],
            )
            threading.Thread(
                target=self._name_joined_game,
                args=(event.data["place_id"],),
                daemon=True,
            ).start()
        elif event.name in (logevents.UDMUX, logevents.JOINED):
            ip = event.data["server_ip"]
            join = self._activity_join
            if join:
                self._activity_join = (join[0], join[1], ip)
            if self.settings.get("server_location_lookup", False):
                threading.Thread(
                    target=self._locate_joined_server, args=(ip, join), daemon=True
                ).start()
        elif event.name == logevents.CRASH:
            print("Roblox crash report line:", event.data.get("text", "")[:200])

    def join_place(self, place_id, job_id=None):
        """Start a game: in the preferred region when that is on and no
        particular server is asked for, else through Roblox's own pick."""
        if self.session or self.busy:
            dialogs._toast(self.toasts, _("Roblox is already running or busy"))
            return
        if not self.rosemary_selected() and not core.installed_version():
            self.show_setup("overview")
            return
        key = self.settings.get("region_preferred", "")
        if job_id is None and self.settings.get("region_join", False) and key:
            dialogs._toast(
                self.toasts,
                _("Looking for a server in {region}…").format(
                    region=self._region_name(key)
                ),
            )
            avoid = {
                s.get("job_id")
                for s in history.load(core.CONFIG_DIR)
                .get(str(place_id), {})
                .get("servers", [])
            }

            def work():
                server = region.pick_server(place_id, key, avoid=avoid)
                GLib.idle_add(self._region_picked, place_id, server, key)

            threading.Thread(target=work, daemon=True).start()
            return
        self.launch(history.rejoin_uri(place_id, job_id))

    def _region_picked(self, place_id, server, key):
        if self.session or self.busy:
            return False
        if server is None:
            dialogs._toast(
                self.toasts,
                _("No server found in {region}; joining normally").format(
                    region=self._region_name(key)
                ),
            )
        self.launch(history.rejoin_uri(place_id, server))
        return False

    @staticmethod
    def _region_name(key):
        country, city = region.split_region(key)
        return f"{city}, {country}" if city else (country or "")

    def _name_joined_game(self, place_id):
        try:
            info = discord.fetch_game_info(int(place_id))
        except Exception:
            return
        if (
            info
            and self.settings.get("game_update_notice", True)
            and info.get("universe_id")
        ):
            age = game_meta.recently_updated(
                game_meta.last_updated(info["universe_id"]), time.time()
            )
            if age is not None:
                GLib.idle_add(
                    dialogs._toast,
                    self.toasts,
                    _(
                        "{name} was updated {n} h ago. If something looks off, "
                        "that is the likely reason."
                    ).format(name=info.get("name", _("This game")), n=max(1, int(age))),
                )
        if info and info.get("name"):
            data = history.load(core.CONFIG_DIR)
            entry = data.get(str(place_id))
            if entry is not None and entry.get("servers"):
                server = entry["servers"][0]
                history.record(
                    core.CONFIG_DIR,
                    place_id,
                    server["job_id"],
                    name=info["name"],
                    server_ip=server.get("server_ip"),
                    location=server.get("location"),
                    now=server.get("joined_at"),
                )
                GLib.idle_add(self._refresh_history)

    def _locate_joined_server(self, ip, join):
        try:
            location = core.query_server_location(ip)
        except Exception:
            return
        if not location:
            return
        if join:
            history.set_location(core.CONFIG_DIR, join[0], join[1], ip, location)
        GLib.idle_add(self.notify_server_location, location, ip)
        GLib.idle_add(self._refresh_history)

    def _refresh_history(self):
        self.servers_page.refresh()
        return False

    def _settle_mod_guard(self, status):
        """A crash soon after new mods were applied sets those mods aside."""
        if not self.settings.get("enable_custom_mods", True):
            return
        try:
            crashed = status not in (0, -1)
            set_aside = modguard.end_session(
                core.CONFIG_DIR, mods.MODS_DIR, crashed=crashed
            )
        except Exception as error:
            print("Mod crash guard failed:", error)
            return
        if set_aside:
            modguard.clear_notice(core.CONFIG_DIR)
            dialogs._toast(
                self.toasts,
                _(
                    "Roblox crashed soon after new mods were applied. Set aside: {names}"
                ).format(names=", ".join(set_aside)),
            )
            self.mods_page.refresh_guard()

    def _watch_result(self, session, status):
        """Apply worker results on GTK's thread after polling/cleanup returns."""
        if self._watch_inflight is session:
            self._watch_inflight = None
        if self.session is not session:
            return False
        self._show_transport_warning(session)
        if status is None:
            self._count_playtime()
            if self.play_page.stack.get_visible_child_name() == "logs":
                self.play_page.logs_view.update()
            # Retry RPC connection if Discord was launched after the game
            if self.settings.get("discord_rpc", True):
                self._retry_rpc()
            return False
        self._count_playtime()
        self.session = None
        self._stop_web()
        self._stop_rpc()
        self._stop_activity()
        self.play_page.logs_view.stop_server_location_polling()
        self.settings.save()
        self._settle_mod_guard(status)
        # Roblox's macOS updater cannot reliably replace the host bundle through
        # Darling. It exits with success/a quit sentinel, so this is independent
        # of the frontend exit code and must precede pending-URI auto-retries.
        # Rosemary's client log lines are in its launch log.
        game_log = session.game_log() if hasattr(session, "game_log") else None
        reason = core.exit_reason(game_log or self.last_log)
        if reason == "update_required" and hasattr(session, "game_log"):
            self.pending_uri = (
                self.pending_uri or uri_handoff.peek_pending() or session.launch_uri
            )
            self._rosemary_update_and_relaunch()
            return False
        if reason == "update_required":
            target = core.required_client_update(self.last_log)
            if target and "channel" in target:
                self.set_setting("roblox_channel", target["channel"])
            self.pending_uri = (
                self.pending_uri or uri_handoff.peek_pending() or session.launch_uri
            )
            self.set_visible(True)
            self.present()
            self._roblox_update_required_dialog(target)
            return False
        if reason == "mangohud_fdinfo" and status not in (0, -1):
            # Stop queued protocol retries until the user can disable the
            # crashing overlay; otherwise the same join immediately crashes.
            self.pending_uri = (
                self.pending_uri or uri_handoff.peek_pending() or session.launch_uri
            )
            self.set_visible(True)
            self.present()
            self._mangohud_crash_dialog()
            return False
        pending = self.pending_uri or uri_handoff.peek_pending()
        if pending:
            self.pending_uri = pending
            GLib.idle_add(self.handle_uri, pending)
        failed = status not in (0, -1)
        # A failure is always shown, even with the launcher set to stay closed.
        if (
            failed
            or self.settings_open
            or self.settings.get("open_settings_after_exit", False)
            or self._transport_warning_dialog is not None
        ):
            self.present_settings()
        else:
            self.get_application().quit()
        if failed:
            if reason == "captcha":
                self._captcha_dialog()
            elif reason == "x11_broken":
                self._x11_broken_dialog()
            else:
                dialogs._toast(
                    self.toasts, _("Roblox exited with code {status}", status=status)
                )
        return False

    def _count_playtime(self):
        played = int(time.monotonic() - self._playtime_clock)
        self.settings.stage("playtime_seconds", self._playtime_before_session + played)
        if time.monotonic() - self._playtime_saved_at > 15:
            self._playtime_saved_at = time.monotonic()
            self.settings.save()

    def _retry_rpc(self):
        if self.rpc is not None and self.rpc.connected:
            return
        if time.monotonic() < self._rpc_retry_at:
            return
        self._rpc_retry_at = time.monotonic() + 5
        self._start_rpc()

    def _mangohud_crash_dialog(self):
        dialog = Adw.AlertDialog(
            heading=_("MangoHud overlay crashed"),
            body=_(
                "MangoHud could not read process statistics from /proc. Turn off the overlay, then launch Roblox again."
            ),
        )
        dialog.add_response("close", _("Close"))
        dialog.add_response("disable", _("Turn off MangoHud"))
        dialog.set_default_response("disable")
        dialog.set_close_response("close")

        def response(_dialog, choice):
            if choice == "disable":
                self.set_setting("mangohud", False)
                dialogs._toast(self.toasts, _("MangoHud is off for the next launch"))

        dialog.connect("response", response)
        dialog.present(self)

    def _roblox_update_required_dialog(self, target=None):
        self._update_required_dialog_active = True
        body = _(
            "Roblox closed to install a required update. Let Quadra download the current client, then try launching again."
        )
        if target and target.get("upload"):
            if "channel" in target:
                body = _(
                    "Roblox requires version {version} on channel {channel}. Quadra will download that exact client, then try launching again.",
                    version=target["version"],
                    channel=target["channel"] or _("Production"),
                )
            else:
                body = _(
                    "Roblox requires version {version}. Quadra will download that exact client, then try launching again.",
                    version=target["version"],
                )
        dialog = Adw.AlertDialog(heading=_("Roblox needs an update"), body=body)
        dialog.add_response("later", _("Later"))
        dialog.add_response("update", _("Update Roblox"))
        dialog.set_response_appearance("update", Adw.ResponseAppearance.SUGGESTED)

        def response(_dialog, result):
            self._update_required_dialog_active = False
            if result == "update":
                self.stack.set_visible_child_name("settings")
                self.settings_page.set_tab("roblox")
                # Gated channels can return 401 to the launcher's anonymous
                # check. The signed-in client already reported its deployment.
                if target and target.get("upload"):
                    self.settings_page.install_update(
                        target["upload"],
                        expected_version=target["version"],
                        channel=target.get("channel"),
                    )
                else:
                    self.settings_page.check_updates(
                        install=True,
                        force_download=True,
                        channel=target.get("channel", "") if target else "",
                    )

        dialog.connect("response", response)
        dialog.present(self)

    def _x11_broken_dialog(self):
        has_raw = self.settings.get("raw_mouse", True)
        if has_raw:
            dialog = Adw.AlertDialog(
                heading=_("X11 Connection Lost"),
                body=_(
                    "The game crashed because the X11 connection was broken. "
                    "This usually happens when raw mouse input overloads the display server with events. "
                    "Would you like to disable raw mouse input?"
                ),
            )
            dialog.add_response("cancel", _("Keep Enabled"))
            dialog.add_response("disable", _("Disable Raw Mouse"))
            dialog.set_response_appearance("disable", Adw.ResponseAppearance.SUGGESTED)

            def on_response(_d, result):
                if result == "disable":
                    self.set_setting("raw_mouse", False)
                    dialogs._toast(self.toasts, _("Raw mouse input disabled"))

            dialog.connect("response", on_response)
            dialog.present(self)
        else:
            dialogs._error_dialog(
                self,
                _("X11 Connection Lost"),
                _(
                    "The game crashed because the X11 connection was broken (explicit kill or server shutdown)."
                ),
            )

    def stop(self):
        session = self.session
        if session is not None and hasattr(session, "stop_game"):
            threading.Thread(target=session.stop_game, daemon=True).start()
        else:
            threading.Thread(target=core.stop_roblox, daemon=True).start()

    def _find_last_log(self):
        if not core.LOGS.exists():
            return None
        logs = sorted(core.LOGS.glob("launch-*.log"), key=lambda p: p.stat().st_mtime)
        return logs[-1] if logs else None

    def open_last_log(self):
        log_path = (
            (self.session.log_path if self.session else None)
            or self.last_log
            or self._find_last_log()
        )
        if log_path:
            self.last_log = log_path
            self.play_page.show_logs(log_path)
            return
        self.open_external_log(log_path)

    def open_external_log(self, log_path=None):
        path = (
            log_path
            or (self.session.log_path if self.session else None)
            or self.last_log
            or self._find_last_log()
        )
        if not path or not path.exists():
            dialogs._toast(self.toasts, _("No log found"))
            return
        try:
            Gio.AppInfo.launch_default_for_uri(path.as_uri(), None)
        except (GLib.Error, OSError) as error:
            dialogs._toast(
                self.toasts, _("Could not open the log: {error}", error=error)
            )
