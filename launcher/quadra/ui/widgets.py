"""Small widgets and layout helpers shared by the pages."""

from gi.repository import Adw, GLib, Gtk, Pango

from .. import fflag_allowlist, theme
from ..i18n import _

__all__ = [
    "ComboRow",
    "_add_tabs",
    "_button_row",
    "_follow_setting",
    "_ignored_marker",
    "_page_sidebar",
    "bind_combo",
    "bind_spin",
    "bind_switch",
]


def _button_row(title):
    """A clickable row; Adw.ButtonRow needs libadwaita 1.6 (Ubuntu 24.04 has 1.5)."""
    if hasattr(Adw, "ButtonRow"):
        return Adw.ButtonRow(title=title)
    return Adw.ActionRow(title=title, activatable=True)


def _add_tabs(toolbar_view, stack):
    """Puts `stack` into `toolbar_view` under a tab switcher: libadwaita's, or the
    launcher's own tabs with the custom appearances. Returns the stack that swaps
    between the two (see LauncherWindow.apply_launcher_appearance)."""
    top_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
    top_box.set_margin_top(10)
    top_box.set_margin_bottom(10)
    switcher = Adw.ViewSwitcher(stack=stack, policy=Adw.ViewSwitcherPolicy.WIDE)
    switcher.set_halign(Gtk.Align.CENTER)
    top_box.append(switcher)
    appearance_tabs = Gtk.Stack(hhomogeneous=False, vhomogeneous=False)
    appearance_tabs.add_named(top_box, "gnome")
    custom_tabs = Gtk.ScrolledWindow(
        hscrollbar_policy=Gtk.PolicyType.AUTOMATIC,
        vscrollbar_policy=Gtk.PolicyType.NEVER,
        propagate_natural_height=True,
        child=theme.tabs(stack),
    )
    appearance_tabs.add_named(custom_tabs, "custom")
    toolbar_view.add_top_bar(appearance_tabs)
    toolbar_view.set_content(stack)
    return appearance_tabs


class ComboRow(Adw.ComboRow):
    """A combo row whose chosen value stays whole. libadwaita lets a long subtitle take the room, which squeezes
    the value to an ellipsis (an Accuracy of "Fast" read as "…")."""

    TRIES = 60  # frames to wait for libadwaita to build the value's label

    def __init__(self, **properties):
        super().__init__(**properties)
        self.connect("map", self._keep_value_whole)
        self.connect("notify::selected", self._keep_value_whole)

    def _keep_value_whole(self, *_args):
        tries = [self.TRIES]

        def fix(_widget, _clock):
            label = self._value_label()
            if label is not None:
                label.set_ellipsize(Pango.EllipsizeMode.NONE)
                return GLib.SOURCE_REMOVE
            tries[0] -= 1
            return GLib.SOURCE_CONTINUE if tries[0] > 0 else GLib.SOURCE_REMOVE

        self.add_tick_callback(fix)

    def _value_label(self):
        pending = [self]
        while pending:
            widget = pending.pop()
            if isinstance(widget, Gtk.Popover):  # the choices have room of their own
                continue
            if isinstance(widget, Gtk.ListView) and widget.has_css_class("inline"):
                pending = [widget]
                while pending:
                    inner = pending.pop()
                    if isinstance(inner, Gtk.Label):
                        return inner
                    child = inner.get_first_child()
                    while child:
                        pending.append(child)
                        child = child.get_next_sibling()
                return None
            child = widget.get_first_child()
            while child:
                pending.append(child)
                child = child.get_next_sibling()
        return None


def _follow_setting(window, row, handler, key, show):
    def follow(_key, value):
        with row.handler_block(handler):
            show(value)

    window.watch_setting(key, follow)


def bind_switch(window, row, key, default=False):
    row.set_active(bool(window.settings.get(key, default)))
    handler = row.connect(
        "notify::active",
        lambda changed, _pspec: window.set_setting(key, changed.get_active()),
    )
    _follow_setting(
        window, row, handler, key, lambda value: row.set_active(bool(value))
    )
    return row


def bind_combo(window, row, key, codes, default=None):
    codes = list(codes)

    def index_of(value):
        return codes.index(value) if value in codes else 0

    row.set_selected(index_of(window.settings.get(key, default)))
    handler = row.connect(
        "notify::selected",
        lambda changed, _pspec: window.set_setting(key, codes[changed.get_selected()]),
    )
    _follow_setting(
        window, row, handler, key, lambda value: row.set_selected(index_of(value))
    )
    return row


def bind_spin(
    window,
    row,
    key,
    default,
    to_setting=lambda value: value,
    to_row=lambda value: value,
):
    row.set_value(to_row(window.settings.get(key, default)))
    handler = row.connect(
        "notify::value",
        lambda changed, _pspec: window.set_setting(
            key, to_setting(changed.get_value())
        ),
    )
    _follow_setting(
        window, row, handler, key, lambda value: row.set_value(to_row(value))
    )
    return row


def _ignored_marker(row):
    """A warning icon on a flag row, shown while Roblox's allowlist says the
    client ignores the flag. Returns update(name)."""
    icon = Gtk.Image(
        icon_name="dialog-warning-symbolic",
        valign=Gtk.Align.CENTER,
        visible=False,
        tooltip_text=_(
            "Roblox's allowlist does not include this flag, so the client ignores it"
        ),
    )
    icon.add_css_class("warning")
    row.add_prefix(icon)

    def update(name):
        icon.set_visible(bool(name) and not fflag_allowlist.is_allowed(name))

    return update


def _page_sidebar(stack):
    """The sidebar's page list. Adw.ViewSwitcherSidebar needs libadwaita 1.9;
    older ones (Ubuntu 24.04 has 1.5, Debian 13 1.7) get a plain list of the
    same pages."""
    if hasattr(Adw, "ViewSwitcherSidebar"):
        sidebar = Adw.ViewSwitcherSidebar()
        sidebar.set_stack(stack)
        return sidebar
    names = []
    rows = Gtk.ListBox(selection_mode=Gtk.SelectionMode.BROWSE)
    rows.add_css_class("navigation-sidebar")
    pages = stack.get_pages()
    for index in range(pages.get_n_items()):
        page = pages.get_item(index)
        line = Gtk.Box(spacing=12)
        line.append(Gtk.Image(icon_name=page.get_icon_name()))
        line.append(Gtk.Label(label=page.get_title(), xalign=0))
        rows.append(line)
        names.append(page.get_name())

    def selected(_rows, row):
        if row is not None and stack.get_visible_child_name() != names[row.get_index()]:
            stack.set_visible_child_name(names[row.get_index()])

    def follow(*_args):
        name = stack.get_visible_child_name()
        if name in names:
            row = rows.get_row_at_index(names.index(name))
            if rows.get_selected_row() is not row:
                rows.select_row(row)

    rows.connect("row-selected", selected)
    stack.connect("notify::visible-child-name", follow)
    follow()
    return rows
