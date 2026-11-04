"""The Mods page."""

from pathlib import Path

from gi.repository import Adw, Gio, Gtk

from .. import modguard, mods
from ..i18n import _
from . import dialogs, widgets

__all__ = [
    "ModsPage",
]


class ModsPage(Adw.Bin):
    def __init__(self, window):
        super().__init__()
        self.window = window
        settings = window.settings

        page = Adw.PreferencesPage()

        # 1. Sound presets
        sounds_group = Adw.PreferencesGroup(title=_("Sound presets"))

        death_choices = [
            ("default", _("Default (Roblox)")),
            ("classic_oof", _("Classic OOF")),
            ("custom", _("Custom sound")),
        ]
        self._death_codes = [c[0] for c in death_choices]
        cur_death = settings.get("mod_death_sound", "default")
        cur_death_idx = (
            self._death_codes.index(cur_death) if cur_death in self._death_codes else 0
        )

        self.death_sound_row = widgets.ComboRow(
            title=_("Death sound"),
            subtitle=_("Choose the sound played when your character resets or dies"),
            model=Gtk.StringList.new([c[1] for c in death_choices]),
            selected=cur_death_idx,
        )
        self.death_sound_row.connect("notify::selected", self._on_death_sound_selected)
        sounds_group.add(self.death_sound_row)

        self.custom_death_row = Adw.ActionRow(
            title=_("Custom death sound file"),
            subtitle=settings.get("mod_custom_death_sound") or _("No file chosen"),
            visible=(cur_death == "custom"),
        )
        custom_death_btn = Gtk.Button(label=_("Choose…"), valign=Gtk.Align.CENTER)
        custom_death_btn.connect(
            "clicked", lambda *_args: self._choose_custom_death_sound()
        )
        self.custom_death_row.add_suffix(custom_death_btn)
        sounds_group.add(self.custom_death_row)

        old_sounds_switch = Adw.SwitchRow(
            title=_("Classic movement sounds"),
            subtitle=_(
                "Restores 2006-2014 walking, jumping, getting up, and silent landing sounds"
            ),
            active=settings.get("mod_old_character_sounds", False),
        )
        old_sounds_switch.connect(
            "notify::active",
            lambda row, _pspec: window.set_setting(
                "mod_old_character_sounds", row.get_active()
            ),
        )
        sounds_group.add(old_sounds_switch)

        page.add(sounds_group)

        # 2. Mouse cursors
        cursors_group = Adw.PreferencesGroup(title=_("Mouse cursors"))

        cursor_choices = [
            ("default", _("Default")),
            ("2006", _("2006 Classic")),
            ("2013", _("2013 Retro")),
            ("dot", _("Black & White Dot")),
            ("purple_cross", _("Purple Cross")),
            ("custom", _("Custom cursor")),
        ]
        self._cursor_codes = [c[0] for c in cursor_choices]
        cur_cursor = settings.get("mod_cursor_type", "default")
        cur_cursor_idx = (
            self._cursor_codes.index(cur_cursor)
            if cur_cursor in self._cursor_codes
            else 0
        )

        self.cursor_row = widgets.ComboRow(
            title=_("Cursor style"),
            subtitle=_("Replaces in-game mouse cursors"),
            model=Gtk.StringList.new([c[1] for c in cursor_choices]),
            selected=cur_cursor_idx,
        )
        self.cursor_row.connect("notify::selected", self._on_cursor_selected)
        cursors_group.add(self.cursor_row)

        self.custom_cursor_row = Adw.ActionRow(
            title=_("Custom cursor file or folder"),
            subtitle=settings.get("mod_custom_cursor") or _("No file chosen"),
            visible=(cur_cursor == "custom"),
        )
        custom_cursor_btn = Gtk.Button(label=_("Choose…"), valign=Gtk.Align.CENTER)
        custom_cursor_btn.connect(
            "clicked", lambda *_args: self._choose_custom_cursor()
        )
        self.custom_cursor_row.add_suffix(custom_cursor_btn)
        cursors_group.add(self.custom_cursor_row)

        page.add(cursors_group)

        # 3. Typography
        fonts_group = Adw.PreferencesGroup(title=_("Typography"))

        cur_font = settings.get("mod_custom_font", "")
        self.font_row = Adw.ActionRow(
            title=_("Custom font"),
            subtitle=Path(cur_font).name if cur_font else _("No custom font selected"),
        )
        choose_font_btn = Gtk.Button(label=_("Choose font…"), valign=Gtk.Align.CENTER)
        choose_font_btn.connect("clicked", lambda *_args: self._choose_font())
        self.font_row.add_suffix(choose_font_btn)

        clear_font_btn = Gtk.Button(
            icon_name="edit-clear-symbolic", valign=Gtk.Align.CENTER
        )
        clear_font_btn.add_css_class("flat")
        clear_font_btn.set_tooltip_text(_("Clear font"))
        clear_font_btn.connect("clicked", lambda *_args: self._clear_font())
        self.font_row.add_suffix(clear_font_btn)

        fonts_group.add(self.font_row)
        page.add(fonts_group)

        # 4. User modifications
        mods_group = Adw.PreferencesGroup(title=_("User modifications"))

        custom_mods_switch = Adw.SwitchRow(
            title=_("Enable modifications folder"),
            subtitle=_("Overlay files from modifications/ onto the Roblox client"),
            active=settings.get("enable_custom_mods", True),
        )
        custom_mods_switch.connect(
            "notify::active",
            lambda row, _pspec: window.set_setting(
                "enable_custom_mods", row.get_active()
            ),
        )
        mods_group.add(custom_mods_switch)

        open_folder_row = Adw.ActionRow(
            title=_("Open modifications folder"),
            subtitle=_("Drop your custom textures, sounds, and models here"),
        )
        open_folder_btn = Gtk.Button(label=_("Open"), valign=Gtk.Align.CENTER)
        open_folder_btn.connect("clicked", lambda *_args: self._open_mods_folder())
        open_folder_row.add_suffix(open_folder_btn)
        mods_group.add(open_folder_row)

        page.add(mods_group)

        # Units the crash guard set aside after a crash (reversible).
        self.guard_group = Adw.PreferencesGroup(
            title=_("Set aside by the crash guard"),
            description=_(
                "Roblox crashed soon after these were applied. They are renamed .disabled, not deleted."
            ),
        )
        self._guard_rows = []
        page.add(self.guard_group)
        self.refresh_guard()

        # 5. Management
        mgmt_group = Adw.PreferencesGroup(title=_("Management"))

        apply_btn = widgets._button_row(_("Apply mods now"))
        apply_btn.connect("activated", lambda *_args: self._apply_mods_now())
        mgmt_group.add(apply_btn)

        reset_btn = widgets._button_row(_("Reset all mods to default"))
        reset_btn.add_css_class("destructive-action")
        reset_btn.connect("activated", lambda *_args: self._reset_mods())
        mgmt_group.add(reset_btn)

        page.add(mgmt_group)

        self.set_child(page)

    def refresh_guard(self):
        for row in self._guard_rows:
            self.guard_group.remove(row)
        self._guard_rows = []
        for name in modguard.disabled_units(mods.MODS_DIR):
            row = Adw.ActionRow(title=name)
            button = Gtk.Button(label=_("Re-enable"), valign=Gtk.Align.CENTER)
            button.connect(
                "clicked",
                lambda _b, unit=name: (
                    modguard.restore(mods.MODS_DIR, unit),
                    self.refresh_guard(),
                ),
            )
            row.add_suffix(button)
            self.guard_group.add(row)
            self._guard_rows.append(row)
        self.guard_group.set_visible(bool(self._guard_rows))

    def _on_death_sound_selected(self, row, _pspec):
        code = self._death_codes[row.get_selected()]
        self.window.set_setting("mod_death_sound", code)
        self.custom_death_row.set_visible(code == "custom")

    def _on_cursor_selected(self, row, _pspec):
        code = self._cursor_codes[row.get_selected()]
        self.window.set_setting("mod_cursor_type", code)
        self.custom_cursor_row.set_visible(code == "custom")

    def _choose_custom_death_sound(self):
        def chosen(path):
            self.window.set_setting("mod_custom_death_sound", path)
            self.custom_death_row.set_subtitle(path)

        dialogs._choose_file(
            self.window,
            _("Select custom death sound (.ogg)"),
            _("Select"),
            chosen,
            "Audio files (*.ogg)",
            ("*.ogg",),
        )

    def _choose_custom_cursor(self):
        def chosen(path):
            self.window.set_setting("mod_custom_cursor", path)
            self.custom_cursor_row.set_subtitle(path)

        dialogs._choose_file(
            self.window,
            _("Select custom cursor (.png or folder)"),
            _("Select"),
            chosen,
            "PNG Images (*.png)",
            ("*.png",),
        )

    def _choose_font(self):
        def chosen(path):
            self.window.set_setting("mod_custom_font", path)
            self.font_row.set_subtitle(Path(path).name)

        dialogs._choose_file(
            self.window,
            _("Select font (.ttf, .otf)"),
            _("Select"),
            chosen,
            "Font files (*.ttf, *.otf)",
            ("*.ttf", "*.otf"),
        )

    def _clear_font(self):
        self.window.set_setting("mod_custom_font", "")
        self.font_row.set_subtitle(_("No custom font selected"))

    def _open_mods_folder(self):
        folder = mods.ensure_mods_dir()
        try:
            Gio.AppInfo.launch_default_for_uri(folder.as_uri(), None)
        except Exception:
            import subprocess

            subprocess.Popen(["xdg-open", str(folder)])

    def _apply_mods_now(self):
        if not self.window.begin("applying mods"):
            return
        try:
            mods.apply_mods(self.window.settings.snapshot())
            dialogs._toast(self.window.toasts, _("Mods applied"))
        except Exception as e:
            dialogs._error_dialog(self.window, _("Error applying mods"), str(e))
        finally:
            self.window.end(resume_pending=False)

    def _reset_mods(self):
        if not self.window.begin("resetting mods"):
            return
        try:
            mods.restore_all_mods()
            dialogs._toast(self.window.toasts, _("All mods have been reset"))
        except Exception as e:
            dialogs._error_dialog(self.window, _("Error resetting mods"), str(e))
        finally:
            self.window.end(resume_pending=False)
