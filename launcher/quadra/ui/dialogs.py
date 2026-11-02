"""Toasts and the dialogs every page uses: the copyable error dialog and the VR warning."""

from gi.repository import Adw, Gtk

from .. import __version__, core
from ..i18n import _

__all__ = [
    "_toast",
    "_error_dialog",
    "_vr_warning_dialog",
]


def _toast(overlay, text):
    toast = Adw.Toast.new(text)
    # Error texts contain <, > and & (compiler output, paths); as markup they
    # would turn the toast empty.
    toast.set_use_markup(False)
    overlay.add_toast(toast)


def _error_dialog(window, heading, details):
    """Shows the whole error text, selectable and with a copy button, so
    people can send it. Also kept in ~/.cache/quadrahq/last-error.txt."""
    try:
        core.CACHE_DIR.mkdir(parents=True, exist_ok=True)
        core.LAST_ERROR.write_text(f"Quadra {__version__}\n{heading}\n\n{details}\n")
    except OSError:
        pass
    dialog = Adw.AlertDialog(heading=heading)
    view = Gtk.TextView(
        editable=False,
        monospace=True,
        wrap_mode=Gtk.WrapMode.WORD_CHAR,
        top_margin=8,
        bottom_margin=8,
        left_margin=8,
        right_margin=8,
    )
    view.get_buffer().set_text(details)
    scroller = Gtk.ScrolledWindow(
        child=view,
        min_content_height=160,
        max_content_height=360,
        propagate_natural_height=True,
    )
    scroller.add_css_class("card")
    dialog.set_extra_child(scroller)
    dialog.add_response("copy", _("Copy"))
    dialog.add_response("close", _("Close"))
    dialog.set_default_response("close")

    def response(_dialog, result):
        if result == "copy":
            window.get_clipboard().set(f"Quadra {__version__}\n{heading}\n\n{details}")

    dialog.connect("response", response)
    dialog.present(window)


def _vr_warning_dialog(window, accepted):
    """The warning before VR is turned on. VR attaches its device by changing the running client's memory,
    which is what a Roblox cheat does too, so Roblox's anti-cheat could flag it. `accepted` runs when the
    user goes ahead; only then is "Don't show this again" kept, so cancelling never counts as agreeing."""
    dialog = Adw.AlertDialog(
        heading=_("Turn on VR?"),
        body=_(
            "VR mode patches the Roblox client's memory while the game runs. "
            "You could get flagged by Roblox's anti-cheat for it. The chance is low, but it is not "
            "known how low.\n\nTurn it on only if you accept that risk."
        ),
    )
    hide = Adw.SwitchRow(title=_("Don't show this again"))
    group = Adw.PreferencesGroup()
    group.add(hide)
    dialog.set_extra_child(group)
    dialog.add_response("cancel", _("Cancel"))
    dialog.add_response("enable", _("Turn on VR"))
    dialog.set_response_appearance("enable", Adw.ResponseAppearance.DESTRUCTIVE)
    dialog.set_default_response("cancel")
    dialog.set_close_response("cancel")

    def response(_dialog, choice):
        if choice != "enable":
            return
        if hide.get_active():
            window.set_setting("vr_warning_hidden", True)
        accepted()

    dialog.connect("response", response)
    dialog.present(window)
