"""Tray icon: a StatusNotifierItem with a dbusmenu menu (no GTK needed).

GTK 4 has no tray API; panels that support the StatusNotifier protocol (KDE,
waybar, quickshell/DMS, the GNOME AppIndicator extension...) show the item
and its menu. The item registers with org.kde.StatusNotifierWatcher, and
registers again if the watcher appears later (panel started after Quadra).
Left click runs `activate`; the menu comes from `entries()`, a list of
(label, callback, enabled) tuples, with None for a separator, read each time
the menu is opened."""

import logging
import os

import gi

gi.require_version("Gio", "2.0")
from gi.repository import Gio, GLib

log = logging.getLogger("quadra.tray")

ITEM_XML = """
<node>
 <interface name="org.kde.StatusNotifierItem">
  <property name="Category" type="s" access="read"/>
  <property name="Id" type="s" access="read"/>
  <property name="Title" type="s" access="read"/>
  <property name="Status" type="s" access="read"/>
  <property name="WindowId" type="u" access="read"/>
  <property name="IconName" type="s" access="read"/>
  <property name="IconThemePath" type="s" access="read"/>
  <property name="IconPixmap" type="a(iiay)" access="read"/>
  <property name="ItemIsMenu" type="b" access="read"/>
  <property name="Menu" type="o" access="read"/>
  <method name="ContextMenu"><arg type="i" direction="in"/><arg type="i" direction="in"/></method>
  <method name="Activate"><arg type="i" direction="in"/><arg type="i" direction="in"/></method>
  <method name="SecondaryActivate"><arg type="i" direction="in"/><arg type="i" direction="in"/></method>
  <method name="Scroll"><arg type="i" direction="in"/><arg type="s" direction="in"/></method>
  <signal name="NewIcon"/><signal name="NewTitle"/><signal name="NewStatus"><arg type="s"/></signal>
 </interface>
</node>"""

MENU_XML = """
<node>
 <interface name="com.canonical.dbusmenu">
  <property name="Version" type="u" access="read"/>
  <property name="TextDirection" type="s" access="read"/>
  <property name="Status" type="s" access="read"/>
  <property name="IconThemePath" type="as" access="read"/>
  <method name="GetLayout">
   <arg type="i" direction="in"/><arg type="i" direction="in"/><arg type="as" direction="in"/>
   <arg type="u" direction="out"/><arg type="(ia{sv}av)" direction="out"/>
  </method>
  <method name="GetGroupProperties">
   <arg type="ai" direction="in"/><arg type="as" direction="in"/><arg type="a(ia{sv})" direction="out"/>
  </method>
  <method name="GetProperty"><arg type="i" direction="in"/><arg type="s" direction="in"/><arg type="v" direction="out"/></method>
  <method name="Event">
   <arg type="i" direction="in"/><arg type="s" direction="in"/><arg type="v" direction="in"/><arg type="u" direction="in"/>
  </method>
  <method name="EventGroup"><arg type="a(isvu)" direction="in"/><arg type="ai" direction="out"/></method>
  <method name="AboutToShow"><arg type="i" direction="in"/><arg type="b" direction="out"/></method>
  <method name="AboutToShowGroup"><arg type="ai" direction="in"/><arg type="ai" direction="out"/><arg type="ai" direction="out"/></method>
  <signal name="ItemsPropertiesUpdated"><arg type="a(ia{sv})"/><arg type="a(ia{sv})"/></signal>
  <signal name="LayoutUpdated"><arg type="u"/><arg type="i"/></signal>
  <signal name="ItemActivationRequested"><arg type="i"/><arg type="u"/></signal>
 </interface>
</node>"""

ITEM_PATH = "/StatusNotifierItem"
MENU_PATH = "/Menu"
WATCHER = "org.kde.StatusNotifierWatcher"


def icon_pixmap(png_path):
    """[(width, height, ARGB32 bytes)] for the item's IconPixmap, or []."""
    try:
        gi.require_version("GdkPixbuf", "2.0")
        from gi.repository import GdkPixbuf

        pixbuf = GdkPixbuf.Pixbuf.new_from_file(str(png_path))
        if not pixbuf.get_has_alpha():
            pixbuf = pixbuf.add_alpha(False, 0, 0, 0)
        data = bytearray(pixbuf.get_pixels())
        width, height, stride = (
            pixbuf.get_width(),
            pixbuf.get_height(),
            pixbuf.get_rowstride(),
        )
        out = bytearray()
        for row in range(height):
            line = data[row * stride : row * stride + width * 4]
            for i in range(0, len(line), 4):
                out += bytes(
                    (line[i + 3], line[i], line[i + 1], line[i + 2])
                )  # RGBA -> ARGB
        return [(width, height, bytes(out))]
    except Exception as error:
        log.debug("Tray icon pixmap unavailable: %s", error)
        return []


class Tray:
    def __init__(self, title, icon_name, icon_png, activate, entries):
        self.title = title
        self.icon_name = icon_name
        self.pixmap = icon_pixmap(icon_png) if icon_png else []
        self.activate = activate
        self.entries = entries
        self.connection = None
        self.name_id = 0
        self.watch_id = 0
        self.registered = False
        self._revision = 1
        self._items = []
        self._ids = []

    # -- lifecycle
    def start(self):
        try:
            self.connection = Gio.bus_get_sync(Gio.BusType.SESSION, None)
            self.bus_name = f"org.kde.StatusNotifierItem-{os.getpid()}-1"
            Gio.bus_own_name_on_connection(
                self.connection, self.bus_name, Gio.BusNameOwnerFlags.NONE, None, None
            )
            item = Gio.DBusNodeInfo.new_for_xml(ITEM_XML).interfaces[0]
            menu = Gio.DBusNodeInfo.new_for_xml(MENU_XML).interfaces[0]
            self._item_reg = self.connection.register_object(
                ITEM_PATH, item, self._item_call, self._item_property, None
            )
            self._menu_reg = self.connection.register_object(
                MENU_PATH, menu, self._menu_call, self._menu_property, None
            )
        except GLib.Error as error:
            log.info("No tray icon: %s", error)
            return False
        self.watch_id = Gio.bus_watch_name_on_connection(
            self.connection,
            WATCHER,
            Gio.BusNameWatcherFlags.NONE,
            self._watcher_appeared,
            self._watcher_vanished,
        )
        return True

    def stop(self):
        if self.connection is None:
            return
        if self.watch_id:
            Gio.bus_unwatch_name(self.watch_id)
            self.watch_id = 0
        for registration in (
            getattr(self, "_item_reg", 0),
            getattr(self, "_menu_reg", 0),
        ):
            if registration:
                self.connection.unregister_object(registration)
        self._item_reg = self._menu_reg = 0
        self.connection = None
        self.registered = False

    def menu_changed(self):
        self._revision += 1
        if self.connection is None:
            return
        try:
            self.connection.emit_signal(
                None,
                MENU_PATH,
                "com.canonical.dbusmenu",
                "LayoutUpdated",
                GLib.Variant("(ui)", (self._revision, 0)),
            )
        except GLib.Error as error:
            log.debug("Could not announce the tray menu: %s", error)

    def _watcher_appeared(self, connection, name, owner):
        try:
            connection.call(
                WATCHER,
                "/StatusNotifierWatcher",
                WATCHER,
                "RegisterStatusNotifierItem",
                GLib.Variant("(s)", (self.bus_name,)),
                None,
                Gio.DBusCallFlags.NONE,
                3000,
                None,
                self._registered,
            )
        except GLib.Error as error:
            log.info("Could not register the tray icon: %s", error)

    def _registered(self, connection, result):
        try:
            connection.call_finish(result)
            self.registered = True
        except GLib.Error as error:
            log.info("Tray registration failed: %s", error)

    def _watcher_vanished(self, connection, name):
        self.registered = False

    # -- StatusNotifierItem
    def _item_property(self, connection, sender, path, interface, name):
        values = {
            "Category": GLib.Variant("s", "ApplicationStatus"),
            "Id": GLib.Variant("s", "quadra"),
            "Title": GLib.Variant("s", self.title),
            "Status": GLib.Variant("s", "Active"),
            "WindowId": GLib.Variant("u", 0),
            "IconName": GLib.Variant("s", self.icon_name),
            "IconThemePath": GLib.Variant("s", ""),
            "IconPixmap": GLib.Variant(
                "a(iiay)", [(w, h, d) for w, h, d in self.pixmap]
            ),
            "ItemIsMenu": GLib.Variant("b", False),
            "Menu": GLib.Variant("o", MENU_PATH),
        }
        return values.get(name)

    def _item_call(
        self, connection, sender, path, interface, method, params, invocation
    ):
        if method in ("Activate", "SecondaryActivate"):
            GLib.idle_add(self._run, self.activate)
        invocation.return_value(None)

    @staticmethod
    def _run(callback):
        try:
            callback()
        except Exception as error:
            log.warning("Tray action failed: %s", error)
        return False

    # -- dbusmenu
    def _rebuild(self):
        self._items = [entry for entry in self.entries()]
        self._ids = list(range(1, len(self._items) + 1))

    def _props(self, entry):
        if entry is None:
            return {"type": GLib.Variant("s", "separator")}
        label, _callback, enabled = entry
        return {
            "label": GLib.Variant("s", label),
            "enabled": GLib.Variant("b", bool(enabled)),
            "visible": GLib.Variant("b", True),
        }

    def _menu_property(self, connection, sender, path, interface, name):
        return {
            "Version": GLib.Variant("u", 3),
            "TextDirection": GLib.Variant("s", "ltr"),
            "Status": GLib.Variant("s", "normal"),
            "IconThemePath": GLib.Variant("as", []),
        }.get(name)

    def _menu_call(
        self, connection, sender, path, interface, method, params, invocation
    ):
        if method == "GetLayout":
            self._rebuild()
            children = [
                GLib.Variant("(ia{sv}av)", (i, self._props(e), []))
                for i, e in zip(self._ids, self._items)
            ]
            root = (0, {"children-display": GLib.Variant("s", "submenu")}, children)
            invocation.return_value(
                GLib.Variant("(u(ia{sv}av))", (self._revision, root))
            )
        elif method == "GetGroupProperties":
            wanted = params.unpack()[0]
            rows = [
                (i, self._props(e))
                for i, e in zip(self._ids, self._items)
                if not wanted or i in wanted
            ]
            invocation.return_value(GLib.Variant("(a(ia{sv}))", (rows,)))
        elif method == "GetProperty":
            item_id, name = params.unpack()
            props = {}
            if item_id in self._ids:
                props = self._props(self._items[self._ids.index(item_id)])
            value = props.get(name) or GLib.Variant("s", "")
            invocation.return_value(GLib.Variant("(v)", (value,)))
        elif method == "Event":
            item_id, event = params.unpack()[:2]
            self._click(item_id, event)
            invocation.return_value(None)
        elif method == "EventGroup":
            for item_id, event, _data, _stamp in params.unpack()[0]:
                self._click(item_id, event)
            invocation.return_value(GLib.Variant("(ai)", ([],)))
        elif method == "AboutToShow":
            self._rebuild()
            invocation.return_value(GLib.Variant("(b)", (True,)))
        elif method == "AboutToShowGroup":
            self._rebuild()
            invocation.return_value(GLib.Variant("(aiai)", ([], [])))
        else:
            invocation.return_value(None)

    def _click(self, item_id, event):
        if event != "clicked" or item_id not in self._ids:
            return
        entry = self._items[self._ids.index(item_id)]
        if entry is not None and entry[2]:
            GLib.idle_add(self._run, entry[1])
