"""Tray protocol test with a fake StatusNotifierWatcher.

Needs a private session bus:  dbus-run-session -- python3 tests/launcher/tray_test.py
(with PYTHONPATH=launcher). Exits non-zero on failure."""
import sys

from gi.repository import Gio, GLib

from quadra import tray

WATCHER_XML = """<node><interface name="org.kde.StatusNotifierWatcher">
<method name="RegisterStatusNotifierItem"><arg type="s" direction="in"/></method></interface></node>"""

loop = GLib.MainLoop()
registered, clicks, result, layouts = [], [], {}, []
connection = Gio.bus_get_sync(Gio.BusType.SESSION, None)
connection.signal_subscribe(None, "com.canonical.dbusmenu", "LayoutUpdated", tray.MENU_PATH, None,
                            Gio.DBusSignalFlags.NONE,
                            lambda *signal: layouts.append(signal[-1].unpack()))


def watcher_call(conn, sender, path, iface, method, params, invocation):
    registered.append(params.unpack()[0])
    invocation.return_value(None)


connection.register_object("/StatusNotifierWatcher", Gio.DBusNodeInfo.new_for_xml(WATCHER_XML).interfaces[0],
                           watcher_call, None, None)
Gio.bus_own_name_on_connection(connection, tray.WATCHER, Gio.BusNameOwnerFlags.NONE, None, None)

state = {"running": True}
item = tray.Tray("Quadra", "quadrahq", None, lambda: clicks.append("activate"),
                 lambda: [("Open settings", lambda: clicks.append("settings"), True), None,
                          ("Stop Roblox", lambda: clicks.append("stop"), False)])
assert item.start()


def call(path, iface, method, args, signature, then):
    def done(conn, res):
        then(conn.call_finish(res))
    connection.call(item.bus_name, path, iface, method, GLib.Variant(signature, args) if signature else None,
                    None, Gio.DBusCallFlags.NONE, 3000, None, done)


def step_layout():
    assert registered == [item.bus_name], registered

    def got(reply):
        revision, (root_id, props, children) = reply.unpack()
        labels = [c[1].get("label") or c[1].get("type") for c in (v for v in children)]
        result["labels"] = labels
        call(tray.MENU_PATH, "com.canonical.dbusmenu", "Event", (1, "clicked", GLib.Variant("s", ""), 0),
             "(isvu)", lambda _r: call(tray.MENU_PATH, "com.canonical.dbusmenu", "Event",
                                       (3, "clicked", GLib.Variant("s", ""), 0), "(isvu)",
                                       lambda _r2: call(tray.ITEM_PATH, "org.kde.StatusNotifierItem", "Activate",
                                                        (0, 0), "(ii)", lambda _r3: GLib.timeout_add(100, finish))))
    call(tray.MENU_PATH, "com.canonical.dbusmenu", "GetLayout", (0, -1, []), "(iias)", got)
    return False


def finish():
    item.menu_changed()
    GLib.timeout_add(200, lambda: loop.quit() or False)
    return False


GLib.timeout_add(500, step_layout)
GLib.timeout_add(8000, lambda: (print("TIMEOUT"), sys.exit(2)))
loop.run()
print("labels", result.get("labels"), "clicks", clicks)
assert result["labels"] == ["Open settings", "separator", "Stop Roblox"], result
assert clicks == ["settings", "activate"], clicks  # disabled "Stop Roblox" did not fire
assert layouts == [(2, 0)], layouts
item.stop()
print("TRAY OK")
