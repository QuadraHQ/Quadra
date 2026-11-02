"""The Servers page: recent servers and joining by region."""

import threading
import time

from gi.repository import Adw, GLib, Gtk

from .. import core, history, region
from ..i18n import _
from . import dialogs, widgets

__all__ = [
    "ServersPage",
]


class ServersPage(Adw.Bin):
    """Games and servers joined, newest first, with a rejoin button."""

    def __init__(self, window):
        super().__init__()
        self.window = window
        self.page = Adw.PreferencesPage()
        self._build_region_groups(window)
        self.group = Adw.PreferencesGroup(
            title=_("Server history"),
            description=_(
                "Servers you joined from this launcher. Rejoining asks Roblox for the same "
                "server and falls back to a new one if it has closed."
            ),
        )
        self.page.add(self.group)
        clear_group = Adw.PreferencesGroup()
        clear = widgets._button_row(_("Clear history"))
        clear.add_css_class("destructive-action")
        clear.connect("activated", lambda *_args: self._clear())
        clear_group.add(clear)
        self.page.add(clear_group)
        self.empty = Adw.StatusPage(
            icon_name="network-server-symbolic",
            title=_("No servers yet"),
            description=_("Games you join will show up here."),
            vexpand=True,
        )
        self.stack = Gtk.Stack()
        self.stack.add_named(self.page, "list")
        self.stack.add_named(self.empty, "empty")
        self.set_child(self.stack)
        self._rows = []
        self.refresh()

    def _build_region_groups(self, window):
        settings = window.settings
        group = Adw.PreferencesGroup(
            title=_("Join by region"),
            description=_(
                "Picks a server in the region you choose from RoValra's server list (apis.rovalra.com). "
                "RoValra sees the game's id and the region, not your account."
            ),
        )
        enable = Adw.SwitchRow(
            title=_("Prefer a region"), active=settings.get("region_join", False)
        )
        enable.connect(
            "notify::active",
            lambda row, _pspec: window.set_setting("region_join", row.get_active()),
        )
        group.add(enable)
        self.region_row = widgets.ComboRow(
            title=_("Region"), model=Gtk.StringList.new([_("Loading…")])
        )
        self._region_keys = [""]
        self._region_handler = self.region_row.connect(
            "notify::selected", self._region_selected
        )
        group.add(self.region_row)
        by_id = Adw.EntryRow(title=_("Join a game by id or roblox.com link"))
        by_id.set_show_apply_button(True)
        by_id.connect("apply", self._join_typed)
        group.add(by_id)
        self.page.add(group)
        threading.Thread(target=self._load_regions, daemon=True).start()

    def _load_regions(self):
        GLib.idle_add(self._regions_loaded, region.locations(core.CACHE_DIR))

    def _regions_loaded(self, places):
        names, keys = [_("Choose a region")], [""]
        countries = {}
        for code, country, city in places:
            countries.setdefault((code, country), []).append(city)
        for (code, country), cities in sorted(
            countries.items(), key=lambda item: item[0][1]
        ):
            names.append(country)
            keys.append(code)
            if len(cities) > 1:
                for city in cities:
                    names.append(f"{country} · {city}")
                    keys.append(region.region_key(code, city))
        self.region_row.handler_block(self._region_handler)
        self.region_row.set_model(Gtk.StringList.new(names))
        self._region_keys = keys
        current = self.window.settings.get("region_preferred", "")
        self.region_row.set_selected(keys.index(current) if current in keys else 0)
        self.region_row.handler_unblock(self._region_handler)
        return False

    def _region_selected(self, row, _pspec):
        index = row.get_selected()
        if 0 <= index < len(self._region_keys):
            self.window.set_setting("region_preferred", self._region_keys[index])

    def _join_typed(self, row):
        place = region.parse_place(row.get_text())
        if place is None:
            dialogs._toast(self.window.toasts, _("Enter a place id or a roblox.com/games link"))
            return
        self._join(place)

    def refresh(self):
        for row in self._rows:
            self.group.remove(row)
        self._rows = []
        games = history.recent_games(core.CONFIG_DIR, 25)
        self.stack.set_visible_child_name("list" if games else "empty")
        for place_id, game in games:
            servers = [s for s in game.get("servers", []) if isinstance(s, dict)]
            row = Adw.ExpanderRow(
                title=GLib.markup_escape_text(
                    game.get("name") or _("Place {id}").format(id=place_id)
                ),
                subtitle=self._ago(game.get("last_played")),
            )
            play = Gtk.Button(label=_("Play"), valign=Gtk.Align.CENTER)
            play.add_css_class("suggested-action")
            play.connect("clicked", lambda _b, pid=place_id: self._join(pid))
            row.add_suffix(play)
            for server in servers:
                line = Adw.ActionRow(
                    title=GLib.markup_escape_text(
                        server.get("location")
                        or server.get("server_ip")
                        or _("Unknown location")
                    ),
                    subtitle=self._ago(server.get("joined_at")),
                )
                rejoin = Gtk.Button(label=_("Rejoin"), valign=Gtk.Align.CENTER)
                rejoin.connect(
                    "clicked",
                    lambda _b, pid=place_id, job=server.get("job_id"): self._join(
                        pid, job
                    ),
                )
                line.add_suffix(rejoin)
                row.add_row(line)
            self.group.add(row)
            self._rows.append(row)

    @staticmethod
    def _ago(stamp):
        if not isinstance(stamp, (int, float)):
            return ""
        seconds = max(0, time.time() - stamp)
        if seconds < 3600:
            return _("{n} min ago").format(n=int(seconds // 60) or 1)
        if seconds < 86400:
            return _("{n} h ago").format(n=int(seconds // 3600))
        return _("{n} days ago").format(n=int(seconds // 86400))

    def _join(self, place_id, job_id=None):
        self.window.join_place(place_id, job_id)

    def _clear(self):
        history.forget(core.CONFIG_DIR)
        self.refresh()
