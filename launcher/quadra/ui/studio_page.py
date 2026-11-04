"""The Roblox Studio page."""

import threading
from pathlib import Path

from gi.repository import Adw, GLib, Gtk

from .. import core, gpus, studio
from ..i18n import _
from . import dialogs, flags_page, system, widgets

__all__ = [
    "StudioPage",
]


class StudioPage(Adw.Bin):
    """Roblox Studio through Wine: a section of its own, with the settings that belong to it."""

    def __init__(self, window):
        super().__init__()
        self.window = window
        self._releases_loaded = False
        settings = window.settings

        toolbar_view = Adw.ToolbarView()
        self.stack = Adw.ViewStack()
        self.stack.add_titled_with_icon(
            self._general_page(settings),
            "general",
            _("Studio"),
            "applications-development-symbolic",
        )
        self.stack.add_titled_with_icon(
            self._graphics_page(settings),
            "graphics",
            _("Graphics"),
            "video-display-symbolic",
        )
        self.stack.add_titled_with_icon(
            self._updates_page(settings),
            "updates",
            _("Wine and updates"),
            "software-update-available-symbolic",
        )
        self.flags_page = flags_page.StudioFlagsPage(window)
        self.stack.add_titled_with_icon(
            self.flags_page, "flags", _("Fast flags"), "preferences-other-symbolic"
        )
        self.appearance_tabs = widgets._add_tabs(toolbar_view, self.stack)
        self.set_child(toolbar_view)
        self.connect("map", lambda *_args: self._shown())
        window.watch_studio(self.show_studio_status)
        self.show_studio_status(window.studio_status.value)
        self.refresh()

    # -- helpers
    def _in_thread(self, work, done):
        def run():
            try:
                result = work()
                GLib.idle_add(done, result, None)
            except Exception as error:  # shown to the user
                GLib.idle_add(done, None, error)

        threading.Thread(target=run, daemon=True).start()

    def _switch(self, group, key, title, subtitle):
        row = Adw.SwitchRow(title=title, subtitle=subtitle)
        widgets.bind_switch(self.window, row, key, core.DEFAULT_SETTINGS[key])
        group.add(row)
        return row

    def _entry(self, group, key, title, valid, hint):
        """A text setting that is saved when its apply button is pressed."""
        row = Adw.EntryRow(title=title, show_apply_button=True)
        row.set_text(self.window.settings.get(key, ""))

        def apply(changed):
            text = changed.get_text().strip()
            if text and not valid(text):
                changed.add_css_class("error")
                dialogs._toast(self.window.toasts, hint)
                return
            changed.remove_css_class("error")
            self.window.set_setting(key, text)
            changed.set_text(text)

        row.connect("apply", apply)
        group.add(row)
        return row

    def _folder_row(self, title, subtitle, folder):
        row = Adw.ActionRow(title=title, subtitle=subtitle, activatable=True)
        row.add_suffix(Gtk.Image(icon_name="folder-open-symbolic"))

        def open_folder(*_args):
            try:
                Path(folder()).mkdir(parents=True, exist_ok=True)
            except OSError as error:
                dialogs._error_dialog(self.window, _("Could not open the folder"), str(error))
                return
            system._open_uri(self.window, Path(folder()).as_uri())

        row.connect("activated", open_folder)
        return row

    # -- pages
    def _general_page(self, settings):
        page = Adw.PreferencesPage()

        status = Adw.PreferencesGroup(
            title="Roblox Studio",
            description=_(
                "The Windows version of Studio, run through Wine. Quadra keeps Wine and Studio "
                "up to date when you start it."
            ),
        )
        self.version_row = Adw.ActionRow(title=_("Installed version"))
        self.wine_status_row = Adw.ActionRow(title="Wine")
        self.webview_status_row = Adw.ActionRow(title="WebView2")
        for row in (self.version_row, self.wine_status_row, self.webview_status_row):
            row.set_subtitle_selectable(True)
            status.add(row)
        self.launch_row = widgets._button_row(_("Launch Roblox Studio"))
        self.launch_row.add_css_class("suggested-action")
        self.launch_row.connect(
            "activated", lambda *_args: self.window.studio_clicked()
        )
        status.add(self.launch_row)
        self.progress = Gtk.ProgressBar(
            show_text=True,
            margin_top=6,
            margin_bottom=6,
            margin_start=12,
            margin_end=12,
            visible=False,
        )
        status.add(
            Gtk.ListBoxRow(activatable=False, selectable=False, child=self.progress)
        )
        page.add(status)

        behaviour = Adw.PreferencesGroup(title=_("Behavior"))
        self._switch(
            behaviour,
            "studio_webview",
            _("Microsoft WebView2"),
            _(
                "Studio's sign-in, Toolbox and Creator Hub pages need it. Installed on the next start."
            ),
        )
        self._switch(
            behaviour,
            "studio_discord",
            _("Discord presence"),
            _("Shows the place you are building and whether you are editing a script."),
        )
        self._switch(
            behaviour,
            "studio_theme",
            _("Follow the system theme"),
            _(
                "Light or dark, like the rest of your desktop. Applies on the next start."
            ),
        )
        self._switch(
            behaviour,
            "studio_backup",
            _("Back up sign-in and settings"),
            _(
                "Keeps them outside Wine's prefix, so recreating the prefix does not sign you out."
            ),
        )
        page.add(behaviour)

        files = Adw.PreferencesGroup(title=_("Files"))
        files.add(
            self._folder_row(
                _("Overlay folder"),
                _(
                    "Files here are copied over Studio's own at every start, such as cursors or sounds."
                ),
                lambda: studio.OVERLAY,
            )
        )
        files.add(
            self._folder_row(
                _("Logs"), _("Studio's output, newest ten starts."), lambda: core.LOGS
            )
        )
        files.add(
            self._folder_row(
                _("Studio folder"),
                _("Wine, its prefix and Studio itself."),
                lambda: studio.ROOT,
            )
        )
        page.add(files)
        return page

    def _graphics_page(self, settings):
        page = Adw.PreferencesPage()

        renderer = Adw.PreferencesGroup(
            title=_("Rendering"),
            description=_(
                "Applies the next time Studio starts. DXVK draws Direct3D 11 through Vulkan."
            ),
        )
        self.renderer_codes = list(studio.RENDERERS)
        labels = [
            _("DXVK (recommended)"),
            _("DXVK-Sarek (older graphics cards)"),
            _("Wine's Direct3D (OpenGL)"),
            _("Wine's Direct3D, feature level 10 (OpenGL)"),
            "Vulkan",
        ]
        self.renderer_row = widgets.ComboRow(
            title=_("Renderer"), model=Gtk.StringList.new(labels)
        )
        current = studio.Options(settings).renderer
        self.renderer_row.set_selected(self.renderer_codes.index(current))
        self.renderer_row.connect("notify::selected", self._renderer_selected)
        renderer.add(self.renderer_row)
        self.renderer_note = Adw.ActionRow(
            title=_("The Vulkan fix is not built"),
            subtitle=_(
                "Studio's Vulkan renderer needs a small Vulkan layer that is built with the shim, "
                "which needs the Vulkan development headers. Rebuild the shim after installing them."
            ),
        )
        self.renderer_note.add_prefix(Gtk.Image(icon_name="dialog-warning-symbolic"))
        renderer.add(self.renderer_note)
        self.renderer_note.set_visible(
            current == "vulkan" and not studio.vulkan_layer_built()
        )
        page.add(renderer)

        performance = Adw.PreferencesGroup(title=_("Performance"))
        self._gpu_codes = [""]
        self.gpu_row = widgets.ComboRow(
            title=_("Graphics card"),
            subtitle=_("Detecting graphics cards… Applies on next launch."),
            model=Gtk.StringList.new([_("Automatic")]),
            sensitive=False,
            visible=settings.get("studio_gpu", "") not in ("", "auto"),
        )
        self._gpu_handler = self.gpu_row.connect("notify::selected", self._gpu_selected)
        performance.add(self.gpu_row)
        self._in_thread(gpus.enumerate_gpus, self._gpu_choices_loaded)
        self._switch(
            performance,
            "studio_gamemode",
            "GameMode",
            _(
                "Asks Feral GameMode to tune your system while Studio runs, if it is installed."
            ),
        )
        page.add(performance)
        return page

    def _updates_page(self, settings):
        page = Adw.PreferencesPage()

        wine = Adw.PreferencesGroup(
            title="Wine",
            description=_(
                "Studio runs on Kombucha, the Wine build Vinegar patches for Studio. It is updated "
                "when you start Studio, unless you choose a release."
            ),
        )
        self.wine_tags = [""]
        self.release_row = widgets.ComboRow(
            title=_("Kombucha release"),
            subtitle=_("Recommended: the newest stable one, as Vinegar publishes it."),
            model=Gtk.StringList.new([_("Newest stable")]),
        )
        self._release_handler = self.release_row.connect(
            "notify::selected", self._release_selected
        )
        wine.add(self.release_row)
        self._entry(
            wine,
            "studio_wine_root",
            _("Wine folder of your own"),
            lambda text: (Path(text) / "bin" / "wine").is_file(),
            _("That folder has no bin/wine"),
        )
        page.add(wine)

        deployment = Adw.PreferencesGroup(
            title=_("Studio version"),
            description=_(
                "Leave both empty to follow the newest Studio on your account's channel. A version "
                "looks like version-0123456789abcdef; channels are Roblox's, such as zintegration."
            ),
        )
        self._entry(
            deployment,
            "studio_version",
            _("Stay on version"),
            lambda text: studio.VERSION_PATTERN.fullmatch(text) is not None,
            _("A Studio version looks like version-0123456789abcdef"),
        )
        self._entry(
            deployment,
            "studio_channel",
            _("Channel"),
            lambda text: studio.CHANNEL_PATTERN.fullmatch(text) is not None,
            _("A channel name has letters, digits, . _ - only"),
        )
        page.add(deployment)
        return page

    # -- settings
    def _renderer_selected(self, row, _pspec):
        code = self.renderer_codes[row.get_selected()]
        self.window.set_setting("studio_renderer", code)
        self.renderer_note.set_visible(
            code == "vulkan" and not studio.vulkan_layer_built()
        )

    def _gpu_selected(self, row, _pspec):
        selected = row.get_selected()
        if selected < len(self._gpu_codes):
            self.window.set_setting("studio_gpu", self._gpu_codes[selected])

    def _gpu_choices_loaded(self, adapters, error):
        selection = self.window.settings.get("studio_gpu", "")
        adapters = adapters or []
        codes = ["", *(adapter["id"] for adapter in adapters)]
        labels = [_("Automatic"), *(adapter["label"] for adapter in adapters)]
        if selection not in codes:
            codes.append(selection)
            labels.append(_("Selected graphics card unavailable"))
        self._gpu_codes = codes
        self.gpu_row.handler_block(self._gpu_handler)
        try:
            self.gpu_row.set_model(Gtk.StringList.new(labels))
            self.gpu_row.set_selected(codes.index(selection))
        finally:
            self.gpu_row.handler_unblock(self._gpu_handler)
        self.gpu_row.set_sensitive(True)
        self.gpu_row.set_visible(
            len(adapters) > 1 or selection != "" or error is not None
        )
        self.gpu_row.set_subtitle(
            _(
                "Could not detect graphics cards. Select Automatic to use the system default."
            )
            if error
            else _(
                "Applies on next launch. Automatic keeps your desktop's GPU selection."
            )
        )
        return False

    def _release_selected(self, row, _pspec):
        selected = row.get_selected()
        if selected < len(self.wine_tags):
            self.window.set_setting("studio_wine", self.wine_tags[selected])

    def _shown(self):
        """The release list is fetched once, when the page is first shown."""
        if self._releases_loaded:
            return
        self._releases_loaded = True
        self._in_thread(studio.wine_releases, self._releases_loaded_done)
        self.refresh()

    def _releases_loaded_done(self, tags, error):
        chosen = self.window.settings.get("studio_wine", "")
        tags = list(tags or [])
        if chosen and chosen not in tags:
            tags.append(chosen)
        self.wine_tags = ["", *tags]
        labels = [
            _("Newest stable"),
            *(
                f"{tag} ({_('may break Studio')})"
                if tag.startswith(("unstable", "proton"))
                else tag
                for tag in tags
            ),
        ]
        self.release_row.handler_block(self._release_handler)
        try:
            self.release_row.set_model(Gtk.StringList.new(labels))
            self.release_row.set_selected(self.wine_tags.index(chosen))
        finally:
            self.release_row.handler_unblock(self._release_handler)
        if error:
            self.release_row.set_subtitle(
                _("Could not list the releases: {error}", error=error)
            )
        return False

    # -- state shown, and the busy state the window drives
    def refresh(self):
        state = studio._state()
        self.version_row.set_subtitle(
            studio.installed_version() or _("Not installed yet")
        )
        custom = studio.wine_root(studio.Options(self.window.settings)) != studio.WINE
        self.wine_status_row.set_subtitle(
            _("Your own: {path}", path=self.window.settings.get("studio_wine_root"))
            if custom
            else state.get("wine") or _("Not installed yet")
        )
        self.webview_status_row.set_subtitle(state.get("webview") or _("Not installed"))
        return False

    def show_studio_status(self, status):
        busy = status is not None
        fraction, text = status or (None, "")
        self.launch_row.set_sensitive(not busy)
        self.launch_row.set_title(_("Checking…") if busy else _("Launch Roblox Studio"))
        self.progress.set_visible(fraction is not None)
        if fraction is not None:
            self.progress.set_fraction(fraction)
            self.progress.set_text(text)

    def flush(self):
        self.flags_page.flush()
