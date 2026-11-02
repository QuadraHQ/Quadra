"""The small splash shown while Roblox starts."""

from gi.repository import GLib, Gtk

from .. import core
from ..i18n import _

__all__ = [
    "BootSplash",
]


class BootSplash(Gtk.Window):
    """The small window shown while Roblox starts (logo, status, progress,
    Cancel), instead of the full launcher."""

    def __init__(self, window):
        super().__init__(title="Quadra", decorated=False, resizable=False)
        self.set_default_size(460, 280)
        self.add_css_class("boot-splash")
        self.window = window
        self._pulse_id = 0
        box = Gtk.Box(
            orientation=Gtk.Orientation.VERTICAL,
            spacing=14,
            margin_top=26,
            margin_bottom=18,
            margin_start=34,
            margin_end=34,
            valign=Gtk.Align.CENTER,
        )
        logo = Gtk.Image.new_from_file(str(core.ICONS / "quadrahq-64.png"))
        logo.set_pixel_size(72)
        box.append(logo)
        self.status = Gtk.Label(
            label=_("Starting Roblox…"), css_classes=["boot-status"], margin_top=6
        )
        box.append(self.status)
        self.bar = Gtk.ProgressBar()
        box.append(self.bar)
        buttons = Gtk.Box(spacing=8, halign=Gtk.Align.CENTER, margin_top=6)
        self.skip = Gtk.Button(label=_("Skip"), visible=False)
        self.skip.connect("clicked", lambda *_args: window.skip_preload())
        buttons.append(self.skip)
        self.cancel = Gtk.Button(label=_("Cancel"), width_request=160)
        self.cancel.connect("clicked", lambda *_args: window.cancel_start())
        buttons.append(self.cancel)
        box.append(buttons)
        self.set_child(box)
        self.set_progress(None)

    def set_status(self, text):
        self.status.set_text(text)

    def set_progress(self, fraction):
        """A 0..1 fraction, or None for the moving bar."""
        if fraction is None:
            if not self._pulse_id:
                self._pulse_id = GLib.timeout_add(120, self._pulse)
            return
        if self._pulse_id:
            GLib.source_remove(self._pulse_id)
            self._pulse_id = 0
        self.bar.set_fraction(max(0.0, min(1.0, fraction)))

    def _pulse(self):
        self.bar.pulse()
        return True

    def close_splash(self):
        if self._pulse_id:
            GLib.source_remove(self._pulse_id)
            self._pulse_id = 0
        self.destroy()
