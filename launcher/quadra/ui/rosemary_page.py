"""Settings -> Rosemary: choosing the runtime, and installing Rosemary and its client."""

import threading

from gi.repository import Adw, GLib, Gtk

from ..i18n import _
from ..rosemary import install as rosemary_install
from ..rosemary import runtime as rosemary_runtime
from ..rosemary import xr as rosemary_xr
from . import dialogs, widgets

__all__ = [
    "RosemaryPage",
]


class RosemaryPage(Adw.PreferencesPage):
    """Rosemary: the other macOS translation layer. Choose it as the runtime,
    install it, and install Roblox for it."""

    def __init__(self, window):
        super().__init__()
        self.window = window
        settings = window.settings

        runtime_group = Adw.PreferencesGroup(
            title=_("Runtime"),
            description=_(
                "Which layer runs the macOS Roblox client. Rosemary (microlift/rosemary) is Quadra's "
                "default: it runs an unpatched client, and Quadra installs Rosemary and Roblox for it "
                "when you press Play. Darling is outdated, but still has the most features (Quadra "
                "M2VK, web sign-in)."
            ),
        )
        self.backend_row = widgets.ComboRow(
            title=_("Run Roblox with"),
            model=Gtk.StringList.new(["Darling", "Rosemary"]),
        )
        self.backend_row.set_selected(1 if window.rosemary_selected() else 0)
        self.backend_row.connect("notify::selected", self._backend_selected)
        runtime_group.add(self.backend_row)
        self.add(runtime_group)

        program = Adw.PreferencesGroup(title=_("Rosemary"))
        self._sources = ["quadra", "upstream"]
        self.source_row = widgets.ComboRow(
            title=_("Rosemary build"),
            subtitle=_(
                "Quadra's build adds what VR needs; the regular one is microlift's. Quadra installs the one you pick."
            ),
            model=Gtk.StringList.new(
                [_("Quadra's Rosemary (adds VR)"), _("Regular Rosemary")]
            ),
        )
        self.source_row.set_selected(
            self._sources.index(
                rosemary_install.normalize_source(settings.get("rosemary_source"))
            )
        )
        self.source_row.connect("notify::selected", self._source_selected)
        program.add(self.source_row)
        self.program_row = Adw.ActionRow(title=_("Rosemary program"))
        self.install_button = Gtk.Button(
            label=_("Install"),
            valign=Gtk.Align.CENTER,
            css_classes=["suggested-action"],
        )
        self.install_button.connect("clicked", lambda *_args: self._install_rosemary())
        self.program_row.add_suffix(self.install_button)
        program.add(self.program_row)
        path = Adw.EntryRow(
            title=_("Program path (leave empty to find it automatically)")
        )
        path.set_text(settings.get("rosemary_program", ""))
        path.set_show_apply_button(True)
        path.connect(
            "apply",
            lambda row: (
                window.set_setting("rosemary_program", row.get_text().strip()),
                self.refresh(),
            ),
        )
        program.add(path)
        prefix = Adw.EntryRow(title=_("Prefix folder (empty: Quadra's own)"))
        prefix.set_text(settings.get("rosemary_prefix", ""))
        prefix.set_show_apply_button(True)
        prefix.connect(
            "apply",
            lambda row: window.set_setting("rosemary_prefix", row.get_text().strip()),
        )
        program.add(prefix)
        self.add(program)

        client_group = Adw.PreferencesGroup(title=_("Roblox for Rosemary"))
        self.client_row = Adw.ActionRow(title=_("Installed version"))
        self.client_button = Gtk.Button(
            label=_("Install or update"), valign=Gtk.Align.CENTER
        )
        self.client_button.connect("clicked", lambda *_args: self._install_client())
        self.client_row.add_suffix(self.client_button)
        client_group.add(self.client_row)
        self.rollback_row = Adw.ActionRow(title=_("Previous version"))
        self.rollback_button = Gtk.Button(valign=Gtk.Align.CENTER)
        self.rollback_button.connect("clicked", lambda *_args: self._toggle_rollback())
        self.rollback_row.add_suffix(self.rollback_button)
        client_group.add(self.rollback_row)
        self.progress = Gtk.ProgressBar(show_text=True, visible=False, margin_top=6)
        client_group.add(self.progress)
        self.add(client_group)

        self.vr_row = Adw.ActionRow(title=_("VR support"))
        client_group.add(self.vr_row)

        options = Adw.PreferencesGroup(title=_("Options"))
        engine = widgets.ComboRow(
            title=_("Execution"),
            subtitle=_(
                "Native runs the machine's own code; emulated recompiles x86-64"
            ),
            model=Gtk.StringList.new([_("Automatic"), _("Native"), _("Emulated")]),
        )
        modes = list(rosemary_runtime.ENGINES)
        engine.set_selected(
            modes.index(settings.get("rosemary_engine", "auto"))
            if settings.get("rosemary_engine", "auto") in modes
            else 0
        )
        engine.connect(
            "notify::selected",
            lambda row, _pspec: window.set_setting(
                "rosemary_engine", modes[row.get_selected()]
            ),
        )
        options.add(engine)
        opengl = Adw.SwitchRow(
            title=_("Force OpenGL"),
            subtitle=_("For machines without a Vulkan driver"),
            active=settings.get("rosemary_force_opengl", False),
        )
        opengl.connect(
            "notify::active",
            lambda row, _pspec: window.set_setting(
                "rosemary_force_opengl", row.get_active()
            ),
        )
        options.add(opengl)
        drivers = list(rosemary_runtime.VIDEO_DRIVERS)
        display = widgets.ComboRow(
            title=_("Display server"),
            model=Gtk.StringList.new([_("Default"), "Wayland", "X11 (XWayland)"]),
        )
        display.set_selected(
            drivers.index(settings.get("rosemary_video_driver", ""))
            if settings.get("rosemary_video_driver", "") in drivers
            else 0
        )
        display.connect(
            "notify::selected",
            lambda row, _pspec: window.set_setting(
                "rosemary_video_driver", drivers[row.get_selected()]
            ),
        )
        options.add(display)
        swaps = list(rosemary_runtime.SWAP_INTERVALS)
        vsync = widgets.ComboRow(
            title="VSync",
            model=Gtk.StringList.new([_("Default"), _("Off"), _("On"), _("Adaptive")]),
        )
        vsync.set_selected(
            swaps.index(settings.get("rosemary_swap_interval", ""))
            if settings.get("rosemary_swap_interval", "") in swaps
            else 0
        )
        vsync.connect(
            "notify::selected",
            lambda row, _pspec: window.set_setting(
                "rosemary_swap_interval", swaps[row.get_selected()]
            ),
        )
        options.add(vsync)
        presentation = list(rosemary_runtime.PRESENTATION_MODES)
        present = widgets.ComboRow(
            title=_("Presentation mode"),
            subtitle=_("How frames reach the screen (Vulkan present mode)"),
            model=Gtk.StringList.new(
                [_("Default"), _("Immediate"), "FIFO", _("FIFO relaxed"), "Mailbox"]
            ),
        )
        current = settings.get("rosemary_presentation_mode", "")
        present.set_selected(
            presentation.index(current) if current in presentation else 0
        )
        present.connect(
            "notify::selected",
            lambda row, _pspec: window.set_setting(
                "rosemary_presentation_mode", presentation[row.get_selected()]
            ),
        )
        options.add(present)
        rate = Adw.EntryRow(
            title=_("Refresh rate told to the game (Hz, empty: the display's)")
        )
        rate.set_text(settings.get("rosemary_refresh_rate", ""))
        rate.set_show_apply_button(True)

        def rate_applied(row):
            text = row.get_text().strip()
            if text and not rosemary_runtime.refresh_rate(text):
                row.add_css_class("error")
                dialogs._toast(window.toasts, _("Enter a number of hertz above zero"))
                return
            row.remove_css_class("error")
            window.set_setting("rosemary_refresh_rate", text)

        rate.connect("apply", rate_applied)
        options.add(rate)
        decorations = list(rosemary_runtime.DECORATIONS)
        title_bar = widgets.ComboRow(
            title=_("Title bar"),
            subtitle=_("Who draws the window's title bar"),
            model=Gtk.StringList.new(
                [_("Default"), _("Rosemary (Mac style)"), _("Window manager")]
            ),
        )
        current = settings.get("rosemary_decorations", "")
        title_bar.set_selected(
            decorations.index(current) if current in decorations else 0
        )
        title_bar.connect(
            "notify::selected",
            lambda row, _pspec: window.set_setting(
                "rosemary_decorations", decorations[row.get_selected()]
            ),
        )
        options.add(title_bar)
        menus = list(rosemary_runtime.MENUS)
        menu = widgets.ComboRow(
            title=_("Menu placement"),
            subtitle=_('"Hide the macOS menu bar" in Game settings overrides this'),
            model=Gtk.StringList.new(
                [_("Default"), _("In the title bar"), _("Strip under it"), _("None")]
            ),
        )
        current = settings.get("rosemary_menu", "")
        menu.set_selected(menus.index(current) if current in menus else 0)
        menu.connect(
            "notify::selected",
            lambda row, _pspec: window.set_setting(
                "rosemary_menu", menus[row.get_selected()]
            ),
        )
        options.add(menu)
        for key, title, subtitle in (
            ("rosemary_confine_pointer", _("Keep the pointer inside the window"), None),
            (
                "rosemary_control_edit_keys",
                _("Control works as Command in editing shortcuts"),
                None,
            ),
            (
                "rosemary_plain_ctrl_click",
                _("Control-click is a plain left click"),
                _("Crouching with Control would otherwise be a right click"),
            ),
        ):
            row = Adw.SwitchRow(
                title=title, subtitle=subtitle or "", active=settings.get(key, False)
            )
            row.connect(
                "notify::active",
                lambda r, _pspec, k=key: window.set_setting(k, r.get_active()),
            )
            options.add(row)
        zink = Adw.SwitchRow(
            title=_("VR: OpenGL through Vulkan (zink)"),
            subtitle=_(
                "Roblox's VR needs its OpenGL renderer; this runs it on Vulkan with Mesa's zink"
            ),
            active=settings.get("rosemary_vr_zink", False),
        )
        zink.connect(
            "notify::active",
            lambda row, _pspec: window.set_setting(
                "rosemary_vr_zink", row.get_active()
            ),
        )
        options.add(zink)
        server = Adw.SwitchRow(
            title=_("Process server"),
            subtitle=_("Stopped with the game; turn off to leave it running"),
            active=settings.get("rosemary_process_server", True),
        )
        server.connect(
            "notify::active",
            lambda row, _pspec: window.set_setting(
                "rosemary_process_server", row.get_active()
            ),
        )
        options.add(server)
        self.add(options)
        self.refresh()

    def _backend_selected(self, row, _pspec):
        window = self.window
        window.set_setting(
            "runtime_backend", "rosemary" if row.get_selected() == 1 else "darling"
        )
        window.apply_runtime_state()
        self.refresh()
        # Quadra installs Rosemary and the client itself when they are missing.
        if (
            window.rosemary_selected()
            and not window.rosemary_ready()
            and not window.busy
        ):
            self._install_client()

    def _source(self):
        return rosemary_install.normalize_source(
            self.window.settings.get("rosemary_source")
        )

    def _source_selected(self, row, _pspec):
        window = self.window
        kind = self._sources[row.get_selected()]
        window.set_setting("rosemary_source", kind)
        self.refresh()
        # Quadra installs the build you picked when it is not there yet.
        if (
            not rosemary_runtime.quadra_runners(kind)
            and not (window.settings.get("rosemary_program") or "").strip()
            and not window.busy
        ):
            self._install_rosemary()

    def _check_runner_update(self):
        kind = self._source()
        newest = rosemary_install.latest_version(source=kind)
        have = {
            name.removeprefix("Rosemary ")
            for name, _path in rosemary_runtime.quadra_runners(kind)
        }
        # Quadra's build falls back to the regular one while it has no release: not an update.
        if (
            newest
            and have
            and newest not in have
            and rosemary_install.runner_kind(newest) == kind
        ):
            GLib.idle_add(
                self.install_button.set_label, _("Update to {version}", version=newest)
            )
        return False

    def refresh(self):
        kind = self._source()
        try:
            found = rosemary_runtime.find_rosemary(self.window.settings)
            self.program_row.set_subtitle(rosemary_runtime.describe(found))
            has_chosen = bool(rosemary_runtime.quadra_runners(kind))
            custom = bool((self.window.settings.get("rosemary_program") or "").strip())
            self.install_button.set_label(
                _("Update") if (has_chosen or custom) else _("Install")
            )
            if has_chosen:
                threading.Thread(target=self._check_runner_update, daemon=True).start()
        except rosemary_runtime.RosemaryError as error:
            self.program_row.set_subtitle(str(error).splitlines()[0])
            self.install_button.set_label(_("Install"))
        settings = self.window.settings
        try:
            program = rosemary_runtime.find_rosemary(settings)
            self.vr_row.set_subtitle(
                _("Ready: turn on VR in the VR tab")
                if rosemary_xr.supports_vr(program)
                else _(
                    'This Rosemary cannot load VR. Choose Quadra\'s Rosemary under "Rosemary build".'
                )
            )
        except rosemary_runtime.RosemaryError:
            self.vr_row.set_subtitle(_("Install Rosemary first"))
        self.client_row.set_subtitle(
            rosemary_runtime.installed_version(settings) or _("Not installed")
        )
        previous = rosemary_runtime.previous_build(settings)
        pinned = bool(settings.get("rosemary_pinned_build"))
        self.rollback_row.set_visible(pinned or previous is not None)
        if pinned:
            self.rollback_row.set_subtitle(
                _("Staying on this version; updates are paused")
            )
            self.rollback_button.set_label(_("Follow latest"))
        elif previous is not None:
            self.rollback_row.set_subtitle(previous[0].version or previous[0].guid)
            self.rollback_button.set_label(_("Go back"))

    def _toggle_rollback(self):
        settings = self.window.settings
        if settings.get("rosemary_pinned_build"):
            self.window.set_setting("rosemary_pinned_build", "")
        else:
            previous = rosemary_runtime.previous_build(settings)
            if previous is not None:
                self.window.set_setting("rosemary_pinned_build", previous[0].key)
        self.refresh()

    def _busy(self, busy):
        for button in (self.install_button, self.client_button):
            button.set_sensitive(not busy)
        self.progress.set_visible(busy)

    def _run(self, work):
        """Run `work(progress)` off the GTK thread with the progress bar."""
        self._busy(True)

        def progress(fraction, text):
            GLib.idle_add(
                lambda: (
                    self.progress.set_fraction(max(0.0, min(1.0, fraction))),
                    self.progress.set_text(text),
                    False,
                )[-1]
            )

        def run():
            error = None
            try:
                work(progress)
            except Exception as failure:
                error = failure
            GLib.idle_add(self._finished, error)

        threading.Thread(target=run, daemon=True).start()

    def _finished(self, error):
        self._busy(False)
        self.refresh()
        if error is not None:
            dialogs._error_dialog(self.window, _("Rosemary"), str(error) or repr(error))
        else:
            dialogs._toast(self.window.toasts, _("Done"))
        return False

    def _install_rosemary(self):
        source = self._source()
        self._run(
            lambda progress: rosemary_install.install_latest(progress, source=source)
        )

    def _install_client(self):
        self._run(
            lambda progress: rosemary_runtime.ensure_ready(
                self.window.settings, progress
            )
        )
