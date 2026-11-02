"""The Info page."""

import threading

from gi.repository import Adw, Gdk, GLib, Gtk

from .. import author
from ..i18n import _
from . import system

__all__ = [
    "ABOUT",
    "InfoPage",
]

ABOUT = (
    "Quadra runs the real Roblox client for macOS on Linux through Darling. "
    "It is not made by Roblox and is not affiliated with it."
)


class InfoPage(Adw.PreferencesPage):
    def __init__(self, window):
        super().__init__(title=_("Info"), icon_name="help-about-symbolic")
        self.window = window

        about = Adw.PreferencesGroup(title="Quadra", description=_(ABOUT))
        self.add(about)

        community = Adw.PreferencesGroup(title=_("Community"))
        for title, icon, uri in system._links():
            row = Adw.ActionRow(title=title, activatable=True)
            row.add_prefix(Gtk.Image(icon_name=icon))
            row.add_suffix(Gtk.Image(icon_name="adw-external-link-symbolic"))
            row.connect("activated", lambda *_args, u=uri: system._open_uri(window, u))
            community.add(row)
        self.add(community)

        made_by = Adw.PreferencesGroup(title=_("Authors"))
        self.avatar = Adw.Avatar(size=48, text=author.NAME, show_initials=True)
        profile = Adw.ActionRow(
            title=author.NAME,
            activatable=True,
            subtitle=_("{user} on Roblox", user="@" + author.ROBLOX_USER),
        )
        profile.add_prefix(self.avatar)
        profile.add_suffix(Gtk.Image(icon_name="adw-external-link-symbolic"))
        profile.connect(
            "activated", lambda *_args: system._open_uri(window, author.PROFILE_URL)
        )
        made_by.add(profile)
        maintainer_avatar = Adw.Avatar(
            size=48, text=author.MAINTAINER, show_initials=True
        )
        try:
            maintainer_avatar.set_custom_image(
                Gdk.Texture.new_from_filename(str(author.MAINTAINER_AVATAR))
            )
        except GLib.Error:
            pass
        maintainer = Adw.ActionRow(
            title=author.MAINTAINER,
            activatable=True,
            subtitle=_("Maintains this version: stability and performance fixes"),
        )
        maintainer.add_prefix(maintainer_avatar)
        maintainer.add_suffix(Gtk.Image(icon_name="adw-external-link-symbolic"))
        maintainer.connect(
            "activated", lambda *_args: system._open_uri(window, author.MAINTAINER_URL)
        )
        made_by.add(maintainer)

        self.ui_contributor_avatar = Adw.Avatar(
            size=48, text=author.UI_CONTRIBUTOR, show_initials=True
        )
        ui_contributor = Adw.ActionRow(
            title=author.UI_CONTRIBUTOR, activatable=True, subtitle=_("Better UI, Mods")
        )
        ui_contributor.add_prefix(self.ui_contributor_avatar)
        ui_contributor.add_suffix(Gtk.Image(icon_name="adw-external-link-symbolic"))
        ui_contributor.connect(
            "activated", lambda *_args: system._open_uri(window, author.UI_CONTRIBUTOR_URL)
        )
        made_by.add(ui_contributor)

        claude = Adw.ActionRow(
            title=_("Assisted with Claude Opus 5.5"),
            activatable=True,
            subtitle=_(
                "Anthropic's AI assisted writing the code together with the authors"
            ),
        )
        claude.add_suffix(Gtk.Image(icon_name="adw-external-link-symbolic"))
        claude.connect(
            "activated",
            lambda *_args: system._open_uri(window, "https://www.anthropic.com/claude"),
        )
        made_by.add(claude)
        self.add(made_by)

        settings = dict(window.settings)
        threading.Thread(
            target=lambda: GLib.idle_add(self._show_avatar, author.avatar(settings)),
            daemon=True,
        ).start()
        threading.Thread(
            target=lambda: GLib.idle_add(
                self._show_tinytosha_avatar, author.tinytosha_avatar()
            ),
            daemon=True,
        ).start()

    def _show_avatar(self, path):
        if path:
            try:
                self.avatar.set_custom_image(Gdk.Texture.new_from_filename(str(path)))
            except GLib.Error:
                pass
        return False

    def _show_tinytosha_avatar(self, path):
        if path:
            try:
                self.ui_contributor_avatar.set_custom_image(
                    Gdk.Texture.new_from_filename(str(path))
                )
            except GLib.Error:
                pass
        return False
