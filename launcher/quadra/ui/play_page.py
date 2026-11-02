"""The Play page."""

import os

from gi.repository import Adw, GLib, Gtk, Pango

from .. import core, theme
from ..i18n import _
from ..rosemary import runtime as rosemary_runtime
from . import dialogs, logs_view, system

__all__ = [
    "PlayPage",
]


class PlayPage(Adw.Bin):
    def __init__(self, window):
        super().__init__()
        self.window = window
        toolbar_view = Adw.ToolbarView()

        self.stack = Adw.ViewStack()
        self.top_box = None

        status = Adw.StatusPage()
        status.set_icon_name("quadrahq")
        status.set_title("Quadra")
        self.status = status

        center_box = Gtk.Box(
            orientation=Gtk.Orientation.VERTICAL, spacing=12, halign=Gtk.Align.CENTER
        )

        self.log_button = Gtk.Button(label=_("Open last log"))
        self.log_button.add_css_class("flat")
        self.log_button.set_visible(False)
        self.log_button.connect("clicked", lambda *_args: window.open_last_log())
        center_box.append(self.log_button)

        links = Gtk.Box(spacing=6, halign=Gtk.Align.CENTER, margin_top=14)
        for title, icon, uri in system._links():
            button = Gtk.Button(icon_name=icon, tooltip_text=title)
            button.add_css_class("flat")
            button.add_css_class("circular")
            button.connect("clicked", lambda *_args, u=uri: system._open_uri(window, u))
            links.append(button)
        center_box.append(links)

        status.set_child(center_box)
        self.appearance_stack = Gtk.Stack()
        self.appearance_stack.add_named(status, "gnome")
        self.stack.add_titled_with_icon(
            self.appearance_stack, "play", _("Play"), "media-playback-start-symbolic"
        )

        self.logs_view = logs_view.GameLogsView(window)
        self.stack.add_titled_with_icon(
            self.logs_view, "logs", _("Logs"), "utilities-terminal-symbolic"
        )

        self.top_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        self.top_box.set_margin_top(10)
        self.top_box.set_margin_bottom(10)
        self.switcher = Adw.ViewSwitcher(
            stack=self.stack, policy=Adw.ViewSwitcherPolicy.WIDE
        )
        self.switcher.set_halign(Gtk.Align.CENTER)
        self.top_box.append(self.switcher)
        self.top_box.set_visible(False)

        toolbar_view.add_top_bar(self.top_box)
        toolbar_view.set_content(self.stack)
        self.stack.connect("notify::visible-child-name", self._on_tab_changed)

        action_bar = Gtk.ActionBar()
        self.action_bar = action_bar
        self.toolbar_view = toolbar_view

        self.playtime_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=6)
        self.playtime_box.set_valign(Gtk.Align.CENTER)
        self.playtime_box.add_css_class("dim-label")
        clock_icon = Gtk.Image(icon_name="preferences-system-time-symbolic")
        self.playtime_label = Gtk.Label()
        self.playtime_box.append(clock_icon)
        self.playtime_box.append(self.playtime_label)
        self.playtime_box.set_tooltip_text(_("Total playtime"))
        action_bar.pack_start(self.playtime_box)

        self.studio_progress = Gtk.ProgressBar(show_text=True, visible=False)
        self.studio_progress.set_size_request(160, -1)
        action_bar.pack_start(self.studio_progress)

        self.play = Gtk.Button(label=_("Play"))
        self.play.add_css_class("suggested-action")
        self.play.add_css_class("pill")
        self.play.set_size_request(140, -1)
        self.play.connect("clicked", lambda *_args: self._on_play_clicked())
        action_bar.pack_end(self.play)

        self.studio = Gtk.Button(label=_("Roblox Studio"))
        self.studio.add_css_class("pill")
        self.studio.set_size_request(130, -1)
        self.studio.connect("clicked", lambda *_args: window.studio_clicked())
        # Studio needs Wine, which the Flatpak does not have yet.
        self.studio.set_visible(not os.path.exists("/.flatpak-info"))
        action_bar.pack_end(self.studio)

        toolbar_view.add_bottom_bar(action_bar)
        # The original status page stays intact for GNOME. Both designs share
        # the real launch controls, log buffer and running session.
        self.dashboard_actions = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        self.appearance_stack.add_named(self._dashboard(), "custom")
        self._custom_appearance = False
        self.set_child(toolbar_view)
        self._darling_status_timer = 0
        self.connect("map", self._watch_darling_status)
        self.connect("unmap", self._pause_darling_status)
        self.refresh()

    def _dashboard(self):
        body = Gtk.Box(
            orientation=Gtk.Orientation.VERTICAL,
            spacing=18,
            css_classes=["play-dashboard"],
        )
        hero = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, css_classes=["play-hero"])
        head = Gtk.Box(
            orientation=Gtk.Orientation.VERTICAL,
            spacing=12,
            css_classes=["play-hero-head"],
        )
        identity = Gtk.Box(spacing=14)
        identity.append(theme.Logo(core.ICONS / "quadrahq-64.png", 48))
        titles = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=3, hexpand=True)
        titles.append(Gtk.Label(label="Quadra", xalign=0, css_classes=["play-title"]))
        identity.append(titles)
        head.append(identity)
        self.dashboard_state = Gtk.Label(
            xalign=0, halign=Gtk.Align.START, css_classes=["play-badge"]
        )
        head.append(self.dashboard_state)
        hero.append(head)
        hero.append(self.dashboard_actions)
        body.append(hero)
        body.append(
            Gtk.Label(
                label=_("YOUR ENVIRONMENT"),
                xalign=0,
                css_classes=["launcher-section-title"],
            )
        )
        facts = Gtk.Box(
            orientation=Gtk.Orientation.VERTICAL, css_classes=["play-facts"]
        )
        self.dashboard_facts = {}
        for key, title in (
            ("client", _("Roblox version")),
            ("renderer", _("Renderer")),
            ("runtime", _("Darling runtime")),
        ):
            row = Gtk.Box(spacing=12, css_classes=["play-fact"])
            row.append(
                Gtk.Label(
                    label=title, xalign=0, hexpand=True, css_classes=["play-fact-title"]
                )
            )
            value = Gtk.Label(
                xalign=1,
                ellipsize=Pango.EllipsizeMode.MIDDLE,
                css_classes=["play-fact-value"],
            )
            value.set_max_width_chars(28)
            row.append(value)
            self.dashboard_facts[key] = value
            facts.append(row)
        body.append(facts)
        shortcuts = Gtk.Box(spacing=8, homogeneous=True)
        for title, icon, callback in (
            (
                _("Settings"),
                "emblem-system-symbolic",
                lambda: self.window.stack.set_visible_child_name("settings"),
            ),
            (
                _("Mods"),
                "application-x-addon-symbolic",
                lambda: self.window.stack.set_visible_child_name("mods"),
            ),
            (_("Logs"), "utilities-terminal-symbolic", lambda: self.show_logs()),
        ):
            content = Gtk.Box(spacing=8, halign=Gtk.Align.CENTER)
            content.append(Gtk.Image(icon_name=icon))
            content.append(Gtk.Label(label=title))
            button = Gtk.Button(child=content, css_classes=["play-quick-action"])
            button.connect("clicked", lambda *_args, action=callback: action())
            shortcuts.append(button)
        body.append(shortcuts)
        links = Gtk.Box(spacing=10, halign=Gtk.Align.CENTER)
        for title, icon, address in system._links():
            button = Gtk.Button(
                icon_name=icon, tooltip_text=title, css_classes=["flat"]
            )
            button.connect(
                "clicked", lambda *_args, u=address: system._open_uri(self.window, u)
            )
            links.append(button)
        body.append(links)
        clamp = Adw.Clamp(child=body, maximum_size=760, tightening_threshold=600)
        return Gtk.ScrolledWindow(child=clamp, hscrollbar_policy=Gtk.PolicyType.NEVER)

    def _refresh_darling_status(self):
        # The badge names the runtime in use: Darling or Rosemary.
        if self.window.rosemary_selected():
            self._darling_running = rosemary_runtime.running()
            name = "Rosemary"
        else:
            self._darling_running = core.darlingserver_running()
            name = "Darling"
        self.dashboard_state.set_label(
            _("{runtime} running", runtime=name)
            if self._darling_running
            else _("{runtime} stopped", runtime=name)
        )
        return True

    def _watch_darling_status(self, *_args):
        self._refresh_darling_status()
        if not self._darling_status_timer:
            self._darling_status_timer = GLib.timeout_add_seconds(
                2, self._refresh_darling_status
            )

    def _pause_darling_status(self, *_args):
        if self._darling_status_timer:
            GLib.source_remove(self._darling_status_timer)
            self._darling_status_timer = 0

    # The Studio button's state, driven by LauncherWindow._update_and_start_studio.
    def set_studio_busy(self, busy, label=None):
        self.studio.set_sensitive(not busy)
        self.studio.set_label(label if busy and label else _("Roblox Studio"))

    def show_studio_progress(self, fraction, text):
        self.studio_progress.set_visible(True)
        self.studio_progress.set_fraction(fraction)
        self.studio_progress.set_text(text)

    def hide_studio_progress(self):
        self.studio_progress.set_visible(False)

    def set_appearance(self, appearance):
        custom = appearance != "gnome"
        if custom != self._custom_appearance:
            if custom:
                self.toolbar_view.remove(self.action_bar)
                self.dashboard_actions.append(self.action_bar)
            else:
                self.dashboard_actions.remove(self.action_bar)
                self.toolbar_view.add_bottom_bar(self.action_bar)
            self._custom_appearance = custom
        self.appearance_stack.set_visible_child_name("custom" if custom else "gnome")
        self.logs_view.set_appearance(appearance)

    def _on_tab_changed(self, stack, _pspec):
        if getattr(self, "top_box", None) is None:
            return
        tab = stack.get_visible_child_name()
        if tab == "logs":
            self.logs_view.update()
        elif tab == "play":
            running = self.window.session is not None
            busy = self.window.busy
            active = running or busy == "starting"
            if not active:
                self.top_box.set_visible(False)

    def show_logs(self, log_path=None):
        if not log_path:
            log_path = (
                self.window.session.log_path if self.window.session else None
            ) or self.window.last_log
        if not log_path or not log_path.exists():
            dialogs._toast(self.window.toasts, _("No log found"))
            return
        self.top_box.set_visible(True)
        child = self.switcher.get_first_child()
        idx = 0
        while child:
            if idx == 1:
                child.set_sensitive(True)
                child.set_tooltip_text(_("View game logs"))
            child = child.get_next_sibling()
            idx += 1
        self.logs_view.reset(log_path)
        self.logs_view.update()
        self.stack.set_visible_child_name("logs")

    def _on_play_clicked(self):
        if self.window.session is not None:
            self.window.stop()
        else:
            self.window.play_clicked()

    def refresh_playtime(self):
        show = self.window.settings.get("show_playtime", True)
        self.playtime_box.set_visible(show)
        if show:
            sec = self.window.settings.get("playtime_seconds", 0)
            self.playtime_label.set_text(core.format_playtime(sec))

    def refresh(self):
        running = self.window.session is not None
        busy = self.window.busy
        active = running or busy == "starting"
        viewing_logs = self.stack.get_visible_child_name() == "logs"

        # Show switcher when game is active or currently viewing logs
        self.top_box.set_visible(active or viewing_logs)

        child = self.switcher.get_first_child()
        idx = 0
        while child:
            if idx == 1:
                child.set_sensitive(active or bool(self.window.last_log))
                if not active and not self.window.last_log:
                    child.set_tooltip_text(_("Game is not running"))
                else:
                    child.set_tooltip_text(_("View game logs"))
            child = child.get_next_sibling()
            idx += 1

        if not active and not self.window.last_log and viewing_logs:
            self.stack.set_visible_child_name("play")

        rosemary = self.window.rosemary_selected()
        version = (
            rosemary_runtime.installed_version(self.window.settings)
            if rosemary
            else core.installed_version()
        )
        parts = [
            _("Roblox {version}", version=version) if version else _("Roblox not found")
        ]
        self._refresh_darling_status()
        runtime_name = "Rosemary" if rosemary else "Darling"
        parts.append(
            _("{runtime} running", runtime=runtime_name)
            if self._darling_running
            else _("{runtime} starts with the game", runtime=runtime_name)
        )
        if version and not running and not core.signed_in():
            parts.append(_("Sign in with Quick Login"))
        self.status.set_description(" · ".join(parts))
        self.dashboard_facts["client"].set_label(version or _("Not installed"))
        renderer = {
            "opengl": "OpenGL",
            "vulkan": "Vulkan / Zink",
            "metal": "Quadra M2VK",
        }.get(self.window.settings.get("renderer", "opengl"), "OpenGL")
        self.dashboard_facts["renderer"].set_label(renderer)
        self.dashboard_facts["runtime"].set_label(
            "Rosemary"
            if rosemary
            else (
                "CiderDarling"
                if core.DARLING_RUNTIME["runtime"] == "ciderdarling"
                else "Darling"
            )
        )

        if running:
            self.play.set_label(_("Stop Roblox"))
            self.play.remove_css_class("suggested-action")
            self.play.add_css_class("destructive-action")
            self.play.set_sensitive(True)
        elif busy == "starting":
            self.play.set_label(_("Starting…"))
            self.play.remove_css_class("destructive-action")
            self.play.add_css_class("suggested-action")
            self.play.set_sensitive(False)
        else:
            ready = self.window.rosemary_ready() if rosemary else bool(version)
            self.play.set_label(_("Play") if ready else _("Install Roblox"))
            self.play.remove_css_class("destructive-action")
            self.play.add_css_class("suggested-action")
            self.play.set_sensitive(not busy)

        self.log_button.set_visible(bool(self.window.last_log) and not running)
        self.refresh_playtime()
