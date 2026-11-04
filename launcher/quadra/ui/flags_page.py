"""The fast flag editors: the client's and Roblox Studio's."""

import json
from pathlib import Path

from gi.repository import Adw, Gdk, GLib, Gtk

from .. import core
from ..i18n import _
from . import dialogs, widgets

__all__ = [
    "PRESETS",
    "FlagsPage",
    "StudioFlagsPage",
]

# Common fast flags. Roblox only honours flags on its client allowlist, so
# some of these may have no effect in a given client version.
PRESETS = [
    {
        "title": "FPS limit",
        "subtitle": "FramerateCap + DFIntTaskSchedulerTargetFps",
        "flag": "DFIntTaskSchedulerTargetFps",
        "kind": "fps",
        "default": 144,
        "min": 30,
        "max": 1000,
    },
    {
        "title": "Graphics quality",
        "subtitle": "DFIntDebugFRMQualityLevelOverride, 1–21",
        "flag": "DFIntDebugFRMQualityLevelOverride",
        "kind": "number",
        "default": 10,
        "min": 1,
        "max": 21,
    },
    {
        "title": "MSAA",
        "subtitle": "FIntDebugForceMSAASamples: 0, 1, 2, 4, 8",
        "flag": "FIntDebugForceMSAASamples",
        "kind": "number",
        "default": 4,
        "min": 0,
        "max": 8,
    },
    {
        "title": "No shadows",
        "subtitle": "FIntRenderShadowIntensity = 0",
        "flag": "FIntRenderShadowIntensity",
        "kind": "fixed",
        "value": 0,
    },
    {
        "title": "No grass",
        "subtitle": "FIntFRMMinGrassDistance / FIntFRMMaxGrassDistance = 0",
        "flag": ["FIntFRMMinGrassDistance", "FIntFRMMaxGrassDistance"],
        "kind": "fixed",
        "value": 0,
    },
    {
        "title": "Low quality terrain",
        "subtitle": "FIntTerrainArraySliceSize = 0",
        "flag": "FIntTerrainArraySliceSize",
        "kind": "fixed",
        "value": 0,
    },
    {
        "title": "Texture quality override",
        "subtitle": "DFIntTextureQualityOverride: 3, 16x AF, No mip skipping",
        "flag": "DFIntTextureQualityOverride",
        "kind": "number",
        "default": 3,
        "min": 0,
        "max": 3,
        # The override is ignored unless this is set too.
        "also": {
            "DFFlagTextureQualityOverrideEnabled": True,
            "FIntDebugTextureManagerSkipMips": -1,
            "DFIntTextureCompositorLowResFactor": 1,
            "DFFlagTextureCompositorHighQualityEnabled": True,
            "FIntDebugForceAnisotropy": 16,
        },
    },
    {
        "title": "Anisotropic filtering (16x)",
        "subtitle": "FIntDebugForceAnisotropy: 16 (sharp textures at angles)",
        "flag": "FIntDebugForceAnisotropy",
        "kind": "number",
        "default": 16,
        "min": 1,
        "max": 16,
    },
    {
        "title": "Force maximum texture resolution",
        "subtitle": "FIntDebugTextureManagerSkipMips: -1 (never downscale mips)",
        "flag": "FIntDebugTextureManagerSkipMips",
        "kind": "number",
        "default": -1,
        "min": -1,
        "max": 0,
    },
]


class FlagsPage(Adw.PreferencesPage):
    def __init__(self, window):
        super().__init__(title=_("Fast flags"), icon_name="preferences-other-symbolic")
        self.window = window
        self.flags = core.load_fast_flags()
        self.preset_flags = set()
        self.preset_setters = {}  # flag -> function(value) that shows it in its row
        self.preset_resetters = []
        self._save_timer = 0

        presets = Adw.PreferencesGroup(
            title=_("Popular"),
            description=_(
                "Roblox only applies flags from its allowlist, some flags may have no effect."
            ),
        )
        for preset in PRESETS:
            presets.add(self._preset_row(preset))
        self.add(presets)

        self.custom = Adw.PreferencesGroup(title=_("Custom flags"))
        add_button = Gtk.Button(icon_name="list-add-symbolic", valign=Gtk.Align.CENTER)
        add_button.add_css_class("flat")
        add_button.set_tooltip_text(_("Add flag"))
        add_button.connect("clicked", lambda *_args: self._add_custom_row("", ""))
        import_button = Gtk.Button(label=_("Import JSON"), valign=Gtk.Align.CENTER)
        import_button.add_css_class("flat")
        import_button.connect("clicked", lambda *_args: self._import_dialog())
        export_button = Gtk.Button(label=_("Export JSON"), valign=Gtk.Align.CENTER)
        export_button.add_css_class("flat")
        export_button.connect("clicked", lambda *_args: self._export_dialog())
        suffix = Gtk.Box(spacing=6)
        suffix.append(import_button)
        suffix.append(export_button)
        suffix.append(add_button)
        self.custom.set_header_suffix(suffix)
        self.add(self.custom)
        self.custom_rows = []
        for name, value in self.flags.items():
            if name not in self.preset_flags:
                self._add_custom_row(name, core.format_flag_value(value))

        file_group = Adw.PreferencesGroup()
        path_row = Adw.ActionRow(title=_("File"), subtitle=str(core.FAST_FLAGS_USER))
        path_row.set_subtitle_selectable(True)
        file_group.add(path_row)

        reset_flags_btn = widgets._button_row(_("Reset all fast flags"))
        reset_flags_btn.add_css_class("destructive-action")
        reset_flags_btn.connect("activated", lambda *_args: self._reset_dialog())
        file_group.add(reset_flags_btn)

        self.add(file_group)

    # -- presets
    def _preset_row(self, preset):
        names = preset["flag"] if isinstance(preset["flag"], list) else [preset["flag"]]
        self.preset_flags.update(names)
        enabled = all(name in self.flags for name in names)
        if preset["kind"] == "fps":
            row = Adw.SpinRow.new_with_range(preset["min"], preset["max"], 1)
            row.set_title(_(preset["title"]))
            row.set_subtitle(preset["subtitle"])
            cap = self.window.settings.get("framerate_cap", 0)
            current = self.flags.get(names[0], cap if cap > 0 else preset["default"])
            row.set_value(
                float(current)
                if str(current).lstrip("-").isdigit()
                else preset["default"]
            )
            switch = Gtk.Switch(
                active=cap > 0 or names[0] in self.flags, valign=Gtk.Align.CENTER
            )
            row.add_suffix(switch)

            def apply(*_args):
                value = int(row.get_value()) if switch.get_active() else 0
                # Written into Roblox's settings when the game starts.
                self.window.set_setting("framerate_cap", value)
                for name in names:
                    if value:
                        self.flags[name] = value
                    else:
                        self.flags.pop(name, None)
                self._save()

            switch.connect("notify::active", apply)
            row.connect("notify::value", lambda *_args: switch.get_active() and apply())

            def set_fps(value):
                if not isinstance(value, (int, float)) or isinstance(value, bool):
                    return False
                row.set_value(value)
                switch.set_active(True)
                apply()
                return True

            for name in names:
                self.preset_setters[name] = set_fps
            self.preset_resetters.append(
                lambda: (switch.set_active(False), row.set_value(preset["default"]))
            )
            return row
        if preset["kind"] == "number":
            also = preset.get("also", {})
            self.preset_flags.update(also)
            row = Adw.SpinRow.new_with_range(preset["min"], preset["max"], 1)
            row.set_title(_(preset["title"]))
            row.set_subtitle(preset["subtitle"])
            current = self.flags.get(names[0], preset["default"])
            row.set_value(
                float(current)
                if str(current).lstrip("-").isdigit()
                else preset["default"]
            )
            switch = Gtk.Switch(active=enabled, valign=Gtk.Align.CENTER)
            row.add_suffix(switch)

            def apply(*_args):
                for name in names:
                    if switch.get_active():
                        self.flags[name] = int(row.get_value())
                    else:
                        self.flags.pop(name, None)
                for name, value in also.items():
                    if switch.get_active():
                        self.flags[name] = value
                    else:
                        self.flags.pop(name, None)
                self._save()

            switch.connect("notify::active", apply)
            row.connect("notify::value", lambda *_args: switch.get_active() and apply())

            def set_number(value):
                if not isinstance(value, (int, float)) or isinstance(value, bool):
                    return False
                row.set_value(value)
                switch.set_active(True)
                apply()
                return True

            for name in names:
                self.preset_setters[name] = set_number
            for name in also:
                # Imported on its own: set along with the value above.
                self.preset_setters[name] = lambda value: True
            self.preset_resetters.append(
                lambda: (switch.set_active(False), row.set_value(preset["default"]))
            )
            return row
        row = Adw.SwitchRow(
            title=_(preset["title"]), subtitle=preset["subtitle"], active=enabled
        )

        def toggle(*_args):
            for name in names:
                if row.get_active():
                    self.flags[name] = preset["value"]
                else:
                    self.flags.pop(name, None)
            self._save()

        row.connect("notify::active", toggle)

        def set_fixed(value, name):
            if value == preset["value"]:
                row.set_active(True)
            else:
                # Not this preset's value: keep it as a plain flag.
                self.flags[name] = value
                self._save()
            return True

        for name in names:
            self.preset_setters[name] = lambda value, n=name: set_fixed(value, n)
        self.preset_resetters.append(lambda: row.set_active(False))
        return row

    def refresh_allowlist(self):
        for entry in self.custom_rows:
            entry["mark"](entry["name"].get_text().strip())

    # -- custom flags
    def _add_custom_row(self, name, value):
        row = Adw.ExpanderRow(title=name or _("New flag"), subtitle=value)
        name_row = Adw.EntryRow(title=_("Name"))
        name_row.set_text(name)
        value_row = Adw.EntryRow(title=_("Value"))
        value_row.set_text(value)
        remove = Gtk.Button(
            label=_("Remove"),
            halign=Gtk.Align.END,
            margin_top=6,
            margin_bottom=6,
            margin_end=12,
        )
        remove.add_css_class("destructive-action")
        row.add_row(name_row)
        row.add_row(value_row)
        holder = Gtk.ListBoxRow(activatable=False, selectable=False)
        holder.set_child(remove)
        row.add_row(holder)
        mark = widgets._ignored_marker(row)
        mark(name)
        entry = {"row": row, "name": name_row, "value": value_row, "mark": mark}

        def changed(*_args):
            row.set_title(name_row.get_text() or _("New flag"))
            row.set_subtitle(value_row.get_text())
            mark(name_row.get_text().strip())
            self._sync_custom()

        name_row.connect("changed", changed)
        value_row.connect("changed", changed)

        def delete(*_args):
            self.custom.remove(row)
            self.custom_rows.remove(entry)
            self._sync_custom()

        remove.connect("clicked", delete)
        self.custom.add(row)
        self.custom_rows.append(entry)
        if not name:
            row.set_expanded(True)

    def _sync_custom(self):
        for name in [n for n in self.flags if n not in self.preset_flags]:
            del self.flags[name]
        for entry in self.custom_rows:
            name = entry["name"].get_text().strip()
            if name and name not in self.preset_flags:
                self.flags[name] = core.parse_flag_value(entry["value"].get_text())
        self._save()

    def _import_dialog(self):
        dialog = Adw.AlertDialog(
            heading=_("Import fast flags"),
            body=_(
                'Paste JSON like {"Flag": value}. Flags are added to the current ones.'
            ),
        )
        view = Gtk.TextView(wrap_mode=Gtk.WrapMode.CHAR, monospace=True)
        view.set_size_request(420, 220)
        scroller = Gtk.ScrolledWindow(child=view, min_content_height=220)
        scroller.add_css_class("card")
        dialog.set_extra_child(scroller)
        dialog.add_response("cancel", _("Cancel"))
        dialog.add_response("import", _("Import"))
        dialog.set_response_appearance("import", Adw.ResponseAppearance.SUGGESTED)

        def response(_dialog, result):
            if result != "import":
                return
            buffer = view.get_buffer()
            text = buffer.get_text(
                buffer.get_start_iter(), buffer.get_end_iter(), False
            )
            try:
                data = json.loads(text)
                if not isinstance(data, dict):
                    raise ValueError
            except ValueError:
                dialogs._toast(self.window.toasts, _("This is not a JSON object with flags"))
                return
            existing = {entry["name"].get_text(): entry for entry in self.custom_rows}
            imported = 0
            for name, value in data.items():
                if name in self.preset_flags:
                    # Popular flags live in their own rows above.
                    imported += self.preset_setters[name](value)
                    continue
                if name in existing:
                    existing[name]["value"].set_text(core.format_flag_value(value))
                else:
                    self._add_custom_row(name, core.format_flag_value(value))
                imported += 1
            self._sync_custom()
            dialogs._toast(self.window.toasts, _("Imported flags: {count}", count=imported))

        dialog.connect("response", response)
        dialog.present(self.window)

    def _export_dialog(self):
        text = json.dumps(self.flags, indent=2)
        dialog = Adw.AlertDialog(
            heading=_("Export fast flags"),
            body=_("Copy flags to clipboard or save to a file."),
        )
        view = Gtk.TextView(wrap_mode=Gtk.WrapMode.NONE, monospace=True, editable=False)
        view.set_size_request(420, 220)
        buffer = view.get_buffer()
        buffer.set_text(text)
        scroller = Gtk.ScrolledWindow(child=view, min_content_height=220)
        scroller.add_css_class("card")
        dialog.set_extra_child(scroller)

        dialog.add_response("close", _("Close"))
        dialog.add_response("copy", _("Copy"))
        dialog.add_response("save", _("Save to file…"))
        dialog.set_response_appearance("copy", Adw.ResponseAppearance.SUGGESTED)

        def response(_dialog, result):
            if result == "copy":
                display = Gdk.Display.get_default()
                if display:
                    display.get_clipboard().set(text)
                    dialogs._toast(self.window.toasts, _("Flags copied to clipboard"))
            elif result == "save":
                self._save_export_file(text)

        dialog.connect("response", response)
        dialog.present(self.window)

    def _save_export_file(self, text):
        def chosen(path):
            try:
                Path(path).write_text(text, encoding="utf-8")
            except OSError as error:
                dialogs._error_dialog(self.window, _("Could not save flags"), str(error))
                return
            dialogs._toast(self.window.toasts, _("Flags saved to {path}", path=Path(path).name))

        dialogs._choose_file(self.window, _("Save fast flags"), _("Save"), chosen,
                             "JSON files (*.json)", ("*.json",), save_as="ClientAppSettings.json")

    def _reset_dialog(self):
        dialog = Adw.AlertDialog(
            heading=_("Reset all fast flags?"),
            body=_(
                "All custom flags and presets will be cleared and reset to default."
            ),
        )
        dialog.add_response("cancel", _("Cancel"))
        dialog.add_response("reset", _("Reset"))
        dialog.set_response_appearance("reset", Adw.ResponseAppearance.DESTRUCTIVE)

        def response(_dialog, result):
            if result == "reset":
                self._reset_all_flags()

        dialog.connect("response", response)
        dialog.present(self.window)

    def _reset_all_flags(self):
        self.flags.clear()
        self.window.set_setting("framerate_cap", 0)
        core.save_fast_flags({})

        for reset_func in self.preset_resetters:
            reset_func()

        for entry in list(self.custom_rows):
            self.custom.remove(entry["row"])
        self.custom_rows.clear()

        dialogs._toast(self.window.toasts, _("All fast flags have been reset"))

    def _save(self):
        """Save shortly after the last change: typing a value or spinning a
        number would otherwise rewrite the file on every keystroke."""
        if self._save_timer:
            GLib.source_remove(self._save_timer)
        self._save_timer = GLib.timeout_add(300, self._save_now)

    def _save_now(self):
        self._save_timer = 0
        try:
            core.save_fast_flags(self.flags)
        except OSError as error:
            dialogs._toast(self.window.toasts, _("Could not save flags: {error}", error=error))
        return False

    def flush(self):
        """Write a pending change now (Roblox reads the file when it starts)."""
        if self._save_timer:
            GLib.source_remove(self._save_timer)
            self._save_now()


class StudioFlagsPage(Adw.PreferencesPage):
    """Fast flags for Roblox Studio (the Player's are under Settings -> Fast flags)."""

    def __init__(self, window):
        super().__init__(title=_("Fast flags"), icon_name="preferences-other-symbolic")
        self.window = window
        self.flags = dict(window.settings.get("studio_fflags") or {})
        self.rows = []
        self._save_timer = 0

        self.group = Adw.PreferencesGroup(
            title=_("Studio flags"),
            description=_(
                "Studio only applies flags from Roblox's allowlist. The renderer chosen under "
                "Graphics sets the graphics flags and wins over these."
            ),
        )
        add_button = Gtk.Button(
            icon_name="list-add-symbolic", valign=Gtk.Align.CENTER, css_classes=["flat"]
        )
        add_button.set_tooltip_text(_("Add flag"))
        add_button.connect("clicked", lambda *_args: self._add_row("", ""))
        import_button = Gtk.Button(
            label=_("Import JSON"), valign=Gtk.Align.CENTER, css_classes=["flat"]
        )
        import_button.connect("clicked", lambda *_args: self._import_dialog())
        export_button = Gtk.Button(
            label=_("Export JSON"), valign=Gtk.Align.CENTER, css_classes=["flat"]
        )
        export_button.connect("clicked", lambda *_args: self._export_dialog())
        suffix = Gtk.Box(spacing=6)
        for button in (import_button, export_button, add_button):
            suffix.append(button)
        self.group.set_header_suffix(suffix)
        self.add(self.group)
        for name, value in self.flags.items():
            self._add_row(name, core.format_flag_value(value))

        reset_group = Adw.PreferencesGroup()
        reset = widgets._button_row(_("Remove all Studio flags"))
        reset.add_css_class("destructive-action")
        reset.connect("activated", lambda *_args: self._reset())
        reset_group.add(reset)
        self.add(reset_group)

    def refresh_allowlist(self):
        for entry in self.rows:
            entry["mark"](entry["name"].get_text().strip())

    def _add_row(self, name, value):
        row = Adw.ExpanderRow(title=name or _("New flag"), subtitle=value)
        name_row = Adw.EntryRow(title=_("Name"))
        name_row.set_text(name)
        value_row = Adw.EntryRow(title=_("Value"))
        value_row.set_text(value)
        remove = Gtk.Button(
            label=_("Remove"),
            halign=Gtk.Align.END,
            margin_top=6,
            margin_bottom=6,
            margin_end=12,
            css_classes=["destructive-action"],
        )
        holder = Gtk.ListBoxRow(activatable=False, selectable=False, child=remove)
        for child in (name_row, value_row, holder):
            row.add_row(child)
        mark = widgets._ignored_marker(row)
        mark(name)
        entry = {"row": row, "name": name_row, "value": value_row, "mark": mark}

        def changed(*_args):
            row.set_title(name_row.get_text() or _("New flag"))
            row.set_subtitle(value_row.get_text())
            mark(name_row.get_text().strip())
            self._sync()

        def delete(*_args):
            self.group.remove(row)
            self.rows.remove(entry)
            self._sync()

        name_row.connect("changed", changed)
        value_row.connect("changed", changed)
        remove.connect("clicked", delete)
        self.group.add(row)
        self.rows.append(entry)
        if not name:
            row.set_expanded(True)

    def _sync(self):
        self.flags = {}
        for entry in self.rows:
            name = entry["name"].get_text().strip()
            if name:
                self.flags[name] = core.parse_flag_value(entry["value"].get_text())
        self._save()

    def _save(self):
        """Save shortly after the last change: typing would otherwise rewrite the file on every key."""
        if self._save_timer:
            GLib.source_remove(self._save_timer)
        self._save_timer = GLib.timeout_add(300, self._save_now)

    def _save_now(self):
        self._save_timer = 0
        self.window.set_setting("studio_fflags", dict(self.flags))
        return False

    def flush(self):
        """Write a pending change now (Studio reads the flags when it starts)."""
        if self._save_timer:
            GLib.source_remove(self._save_timer)
            self._save_now()

    def _import_dialog(self):
        dialog = Adw.AlertDialog(
            heading=_("Import Studio flags"),
            body=_(
                'Paste JSON like {"Flag": value}. Flags are added to the current ones.'
            ),
        )
        view = Gtk.TextView(wrap_mode=Gtk.WrapMode.CHAR, monospace=True)
        view.set_size_request(420, 220)
        scroller = Gtk.ScrolledWindow(child=view, min_content_height=220)
        scroller.add_css_class("card")
        dialog.set_extra_child(scroller)
        dialog.add_response("cancel", _("Cancel"))
        dialog.add_response("import", _("Import"))
        dialog.set_response_appearance("import", Adw.ResponseAppearance.SUGGESTED)

        def response(_dialog, result):
            if result != "import":
                return
            buffer = view.get_buffer()
            text = buffer.get_text(
                buffer.get_start_iter(), buffer.get_end_iter(), False
            )
            try:
                data = json.loads(text)
                if not isinstance(data, dict):
                    raise ValueError
            except ValueError:
                dialogs._toast(self.window.toasts, _("This is not a JSON object with flags"))
                return
            existing = {entry["name"].get_text(): entry for entry in self.rows}
            for name, value in data.items():
                if name in existing:
                    existing[name]["value"].set_text(core.format_flag_value(value))
                else:
                    self._add_row(name, core.format_flag_value(value))
            self._sync()
            dialogs._toast(self.window.toasts, _("Imported flags: {count}", count=len(data)))

        dialog.connect("response", response)
        dialog.present(self.window)

    def _export_dialog(self):
        text = json.dumps(self.flags, indent=2)
        dialog = Adw.AlertDialog(
            heading=_("Export Studio flags"), body=_("Copy the flags to the clipboard.")
        )
        view = Gtk.TextView(wrap_mode=Gtk.WrapMode.NONE, monospace=True, editable=False)
        view.set_size_request(420, 220)
        view.get_buffer().set_text(text)
        scroller = Gtk.ScrolledWindow(child=view, min_content_height=220)
        scroller.add_css_class("card")
        dialog.set_extra_child(scroller)
        dialog.add_response("close", _("Close"))
        dialog.add_response("copy", _("Copy"))
        dialog.set_response_appearance("copy", Adw.ResponseAppearance.SUGGESTED)

        def response(_dialog, result):
            display = Gdk.Display.get_default()
            if result == "copy" and display:
                display.get_clipboard().set(text)
                dialogs._toast(self.window.toasts, _("Flags copied to clipboard"))

        dialog.connect("response", response)
        dialog.present(self.window)

    def _reset(self):
        for entry in list(self.rows):
            self.group.remove(entry["row"])
        self.rows.clear()
        self.flags = {}
        self.window.set_setting("studio_fflags", {})
        dialogs._toast(self.window.toasts, _("All Studio flags have been removed"))
