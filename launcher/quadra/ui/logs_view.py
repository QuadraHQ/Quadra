"""The game log viewer on the Play page."""

import os
import re
import threading

from gi.repository import Gdk, GLib, Gtk, Pango

from .. import core
from ..i18n import _
from . import dialogs

__all__ = [
    "GameLogsView",
]


class GameLogsView(Gtk.Box):
    """Live streaming log viewer for Roblox Player with syntax highlighting and search."""

    READ_BYTES = 64 * 1024
    LINES_PER_UPDATE = 200
    LINE_BYTES = 8192
    VIEW_CHARS = 1024 * 1024
    SEARCH_MATCHES = 2048

    def __init__(self, window):
        super().__init__(orientation=Gtk.Orientation.VERTICAL, spacing=6)
        self.window = window
        self.last_pos = 0
        self.line_count = 0
        self.current_log_path = None
        self.auto_scroll = True
        self.matches = []
        self.current_match_idx = -1
        self._pending = bytearray()
        self._discard_line = False
        self._log_identity = None
        self._drain_id = 0
        self._search_refresh_id = 0
        self._search_limited = False
        self._search_query = ""
        self._server_poll_id = 0
        # A lookup reads the Roblox log and may ask RoValra: never on GTK's
        # thread. Results of an older polling run are dropped.
        self._server_generation = 0
        self._server_lookup = False
        window.watch_setting("server_location_lookup", self._follow_server_location_lookup)

        bar = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=6)
        bar.set_margin_start(16)
        bar.set_margin_end(16)
        bar.set_margin_top(8)

        self.title_label = Gtk.Label(label=_("Game Logs"), css_classes=["heading"])
        bar.append(self.title_label)

        self.status_label = Gtk.Label(
            css_classes=["dim-label", "caption"], margin_start=8
        )
        bar.append(self.status_label)
        self.server_label = Gtk.Label(
            css_classes=["dim-label", "caption"], margin_start=8
        )
        self.server_label.set_visible(False)
        bar.append(self.server_label)

        spacer = Gtk.Box(hexpand=True)
        bar.append(spacer)

        search_btn = Gtk.Button(icon_name="edit-find-symbolic")
        search_btn.add_css_class("flat")
        search_btn.set_tooltip_text(_("Search in logs (Ctrl+F)"))
        search_btn.connect("clicked", lambda *_args: self.toggle_search())
        bar.append(search_btn)

        self.scroll_btn = Gtk.ToggleButton(icon_name="go-bottom-symbolic")
        self.scroll_btn.add_css_class("flat")
        self.scroll_btn.set_tooltip_text(_("Auto-scroll"))
        self.scroll_btn.set_active(True)
        self.scroll_btn.connect("toggled", self._on_scroll_toggled)
        bar.append(self.scroll_btn)

        copy_btn = Gtk.Button(icon_name="edit-copy-symbolic")
        copy_btn.add_css_class("flat")
        copy_btn.set_tooltip_text(_("Copy logs"))
        copy_btn.connect("clicked", self._on_copy_clicked)
        bar.append(copy_btn)

        self.open_btn = Gtk.Button(icon_name="document-edit-symbolic")
        self.open_btn.add_css_class("flat")
        self.open_btn.set_tooltip_text(_("Open in text editor"))
        self.open_btn.connect(
            "clicked", lambda *_args: window.open_external_log(self.current_log_path)
        )
        bar.append(self.open_btn)

        clear_btn = Gtk.Button(icon_name="edit-clear-symbolic")
        clear_btn.add_css_class("flat")
        clear_btn.set_tooltip_text(_("Clear view"))
        clear_btn.connect("clicked", self._on_clear_clicked)
        bar.append(clear_btn)

        self.append(bar)

        # Search revealer bar
        self.search_revealer = Gtk.Revealer(
            reveal_child=False, transition_type=Gtk.RevealerTransitionType.SLIDE_DOWN
        )
        search_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=6)
        search_box.set_margin_start(16)
        search_box.set_margin_end(16)
        search_box.set_margin_top(2)
        search_box.set_margin_bottom(2)

        self.search_entry = Gtk.SearchEntry(hexpand=True)
        self.search_entry.set_placeholder_text(_("Search in logs (Ctrl+F)…"))
        self.search_entry.connect("search-changed", self._on_search_changed)
        self.search_entry.connect("activate", lambda *_args: self._find_next())
        self.search_entry.connect("stop-search", lambda *_args: self.close_search())

        self.search_count_label = Gtk.Label(
            css_classes=["dim-label", "caption"], margin_start=6, margin_end=6
        )

        prev_btn = Gtk.Button(
            icon_name="go-up-symbolic", tooltip_text=_("Previous match")
        )
        prev_btn.add_css_class("flat")
        prev_btn.connect("clicked", lambda *_args: self._find_prev())

        next_btn = Gtk.Button(
            icon_name="go-down-symbolic", tooltip_text=_("Next match")
        )
        next_btn.add_css_class("flat")
        next_btn.connect("clicked", lambda *_args: self._find_next())

        close_btn = Gtk.Button(
            icon_name="window-close-symbolic", tooltip_text=_("Close search")
        )
        close_btn.add_css_class("flat")
        close_btn.connect("clicked", lambda *_args: self.close_search())

        search_box.append(self.search_entry)
        search_box.append(self.search_count_label)
        search_box.append(prev_btn)
        search_box.append(next_btn)
        search_box.append(close_btn)

        self.search_revealer.set_child(search_box)
        self.append(self.search_revealer)

        self.scrolled = Gtk.ScrolledWindow(hexpand=True, vexpand=True)
        self.scrolled.set_margin_start(16)
        self.scrolled.set_margin_end(16)
        self.scrolled.set_margin_bottom(12)
        self.scrolled.add_css_class("card")

        self.text_view = Gtk.TextView()
        self.text_view.set_monospace(True)
        self.text_view.set_editable(False)
        self.text_view.set_cursor_visible(False)
        self.text_view.set_wrap_mode(Gtk.WrapMode.NONE)
        self.text_view.set_left_margin(12)
        self.text_view.set_right_margin(12)
        self.text_view.set_top_margin(12)
        self.text_view.set_bottom_margin(12)
        self.buffer = self.text_view.get_buffer()
        self._end_mark = self.buffer.create_mark(
            None, self.buffer.get_end_iter(), False
        )
        self._search_anchor = self.buffer.create_mark(
            None, self.buffer.get_start_iter(), True
        )

        self.tag_ln = self.buffer.create_tag("log_ln", foreground="#6e6e73")
        self.tag_time = self.buffer.create_tag("log_time", foreground="#77767b")
        self.tag_err = self.buffer.create_tag(
            "log_err", foreground="#ed333b", weight=Pango.Weight.BOLD
        )
        self.tag_warn = self.buffer.create_tag(
            "log_warn", foreground="#e5a50a", weight=Pango.Weight.SEMIBOLD
        )
        self.tag_info = self.buffer.create_tag("log_info", foreground="#3584e4")
        self.tag_success = self.buffer.create_tag(
            "log_success", foreground="#33d17a", weight=Pango.Weight.BOLD
        )
        self.tag_debug = self.buffer.create_tag("log_debug", foreground="#7f848e")
        self.tag_quadra = self.buffer.create_tag(
            "log_quadra", foreground="#c061cb", weight=Pango.Weight.BOLD
        )
        self.tag_match = self.buffer.create_tag(
            "search_match", background="#2a5c9a", foreground="#ffffff"
        )
        self.tag_current = self.buffer.create_tag(
            "search_current", background="#f6d32d", foreground="#000000"
        )
        self._ts_re = re.compile(
            r"^(\d{4}-\d{2}-\d{2}[T\s]\d{2}:\d{2}:\d{2}[^\s,]*)(.*)$"
        )

        self.scrolled.set_child(self.text_view)
        self.append(self.scrolled)

        key_controller = Gtk.EventControllerKey()
        key_controller.connect("key-pressed", self._on_key_pressed)
        self.add_controller(key_controller)

    def _on_key_pressed(self, _controller, keyval, _keycode, state):
        if (state & Gdk.ModifierType.CONTROL_MASK) and keyval in (Gdk.KEY_f, Gdk.KEY_F):
            self.toggle_search()
            return True
        if keyval == Gdk.KEY_Escape and self.search_revealer.get_reveal_child():
            self.close_search()
            return True
        return False

    def set_appearance(self, appearance):
        monochrome = appearance == "monochrome"
        colors = {
            "ln": ("#6e6e73", "#9a9a9a"),
            "time": ("#77767b", "#9a9a9a"),
            "err": ("#ed333b", "#ffffff"),
            "warn": ("#e5a50a", "#bdbdbd"),
            "info": ("#3584e4", "#e8e8e8"),
            "success": ("#33d17a", "#f0f0f0"),
            "debug": ("#7f848e", "#9a9a9a"),
            "quadra": ("#c061cb", "#e8e8e8"),
        }
        for name, pair in colors.items():
            getattr(self, "tag_" + name).set_property("foreground", pair[monochrome])
        self.tag_match.set_property(
            "background", "#2f2f2f" if monochrome else "#2a5c9a"
        )
        self.tag_current.set_property(
            "background", "#e8e8e8" if monochrome else "#f6d32d"
        )

    def toggle_search(self):
        if self.search_revealer.get_reveal_child():
            self.close_search()
        else:
            self.open_search()

    def open_search(self):
        self.search_revealer.set_reveal_child(True)
        self.search_entry.grab_focus()
        if self.search_entry.get_text():
            self._on_search_changed(self.search_entry)

    def close_search(self):
        if self._search_refresh_id:
            GLib.source_remove(self._search_refresh_id)
            self._search_refresh_id = 0
        self.search_revealer.set_reveal_child(False)
        self.buffer.remove_tag(
            self.tag_match, self.buffer.get_start_iter(), self.buffer.get_end_iter()
        )
        self.buffer.remove_tag(
            self.tag_current, self.buffer.get_start_iter(), self.buffer.get_end_iter()
        )
        self.search_count_label.set_text("")
        self.matches = []
        self.current_match_idx = -1
        self._search_query = ""
        self.text_view.grab_focus()

    def _on_search_changed(self, entry):
        if self._search_refresh_id:
            GLib.source_remove(self._search_refresh_id)
            self._search_refresh_id = 0
        self._rebuild_search(entry.get_text().strip())

    def _rebuild_search(self, query, preserve=False):
        self._search_limited = False
        anchor = self.buffer.get_iter_at_mark(self._search_anchor).get_offset()
        self._search_query = query
        self.buffer.remove_tag(
            self.tag_match, self.buffer.get_start_iter(), self.buffer.get_end_iter()
        )
        self.buffer.remove_tag(
            self.tag_current, self.buffer.get_start_iter(), self.buffer.get_end_iter()
        )
        if not query:
            self.search_count_label.set_text("")
            self.matches = []
            self.current_match_idx = -1
            return

        matches = []
        it = self.buffer.get_start_iter()
        while True:
            res = it.forward_search(query, Gtk.TextSearchFlags.CASE_INSENSITIVE, None)
            if not res:
                break
            if len(matches) == self.SEARCH_MATCHES:
                self._search_limited = True
                break
            s, e = res
            self.buffer.apply_tag(self.tag_match, s, e)
            matches.append(s.get_offset())
            it = e

        self.matches = matches
        if matches:
            self.current_match_idx = (
                min(range(len(matches)), key=lambda i: abs(matches[i] - anchor))
                if preserve
                else 0
            )
            self._highlight_current_match(query, scroll=not preserve)
        else:
            self.current_match_idx = -1
            self.search_count_label.set_text(_("No matches"))

    def _highlight_current_match(self, query=None, scroll=True):
        if not self.matches or self.current_match_idx < 0:
            return
        if query is None:
            query = self.search_entry.get_text().strip()
        self.buffer.remove_tag(
            self.tag_current, self.buffer.get_start_iter(), self.buffer.get_end_iter()
        )
        offset = self.matches[self.current_match_idx]
        s = self.buffer.get_iter_at_offset(offset)
        e = self.buffer.get_iter_at_offset(offset + len(query))
        self.buffer.apply_tag(self.tag_current, s, e)
        self.buffer.move_mark(self._search_anchor, s)
        if scroll:
            self.text_view.scroll_to_iter(s, 0.2, False, 0.0, 0.5)
        suffix = "+" if self._search_limited else ""
        self.search_count_label.set_text(
            f"{self.current_match_idx + 1} / {len(self.matches)}{suffix}"
        )

    def _find_next(self):
        if not self.matches:
            return
        self.current_match_idx = (self.current_match_idx + 1) % len(self.matches)
        self._highlight_current_match()

    def _find_prev(self):
        if not self.matches:
            return
        self.current_match_idx = (self.current_match_idx - 1) % len(self.matches)
        self._highlight_current_match()

    def _insert_highlighted_text(self, text: str):
        # The reader budgets LF-delimited rows. Other Unicode/control
        # separators must not turn one row into thousands of GTK insertions.
        lines = text.split("\n")
        if lines and not lines[-1]:
            lines.pop()
        for line in lines:
            line = line.rstrip("\r")
            self.line_count += 1
            ll = line.lower()
            if "[quadra" in ll or "[macoblox" in ll:
                tag = self.tag_quadra
            elif any(
                k in ll
                for k in ("error", "crash", "fatal", "sigsegv", "exception", "abort")
            ):
                tag = self.tag_err
            elif any(k in ll for k in ("warning", "warn")):
                tag = self.tag_warn
            elif any(
                k in ll
                for k in (
                    "! joining game",
                    "game join succeeded",
                    "entered play session",
                )
            ):
                tag = self.tag_success
            elif "info" in ll:
                tag = self.tag_info
            elif any(k in ll for k in ("debug", "flog", "dflog")):
                tag = self.tag_debug
            else:
                tag = None

            end = self.buffer.get_end_iter()
            # Gutter line number prefix
            self.buffer.insert_with_tags(end, f"{self.line_count:5d} │ ", self.tag_ln)

            m = self._ts_re.match(line)
            end = self.buffer.get_end_iter()
            if m:
                self.buffer.insert_with_tags(end, m.group(1), self.tag_time)
                end = self.buffer.get_end_iter()
                rest = m.group(2) + "\n"
                if tag:
                    self.buffer.insert_with_tags(end, rest, tag)
                else:
                    self.buffer.insert(end, rest)
            else:
                if tag:
                    self.buffer.insert_with_tags(end, line + "\n", tag)
                else:
                    self.buffer.insert(end, line + "\n")

    def _refresh_server_location(self):
        if (
            not self.window.settings.get("server_location_lookup", False)
            or self.window.session is None
        ):
            self.stop_server_location_polling()
            return False
        if self._server_lookup:
            return True
        self._server_lookup = True
        generation = self._server_generation

        def lookup():
            try:
                ip, location = core.detect_server_location()
            except Exception as error:  # a lookup must never end the poll loop
                print("Server location lookup failed:", error)
                ip = location = None
            GLib.idle_add(self._show_server_location, generation, ip, location)

        try:
            threading.Thread(target=lookup, daemon=True).start()
        except RuntimeError:
            self._server_lookup = False
        return True

    def _show_server_location(self, generation, ip, location):
        self._server_lookup = False
        if generation != self._server_generation or not self._server_poll_id:
            return False
        if location:
            self.window.notify_server_location(location, ip)
        # Nothing found keeps the last answer: the connection line is written
        # once per join and leaves the searched log tail during a long game.
        if location or ip:
            self.server_label.set_text(f"🌍 {location or ip}")
            self.server_label.set_visible(True)
        return False

    def _follow_server_location_lookup(self, _key, enabled):
        if enabled:
            self.start_server_location_polling()
        else:
            self.stop_server_location_polling()

    def start_server_location_polling(self):
        """Poll while a game runs and the user opted in (it contacts RoValra)."""
        if self._server_poll_id:
            return
        if (
            not self.window.settings.get("server_location_lookup", False)
            or self.window.session is None
        ):
            return
        self._server_generation += 1
        self._server_poll_id = GLib.timeout_add_seconds(
            10, self._refresh_server_location
        )
        self._refresh_server_location()

    def stop_server_location_polling(self):
        if self._server_poll_id:
            GLib.source_remove(self._server_poll_id)
            self._server_poll_id = 0
        self._server_generation += 1
        self.server_label.set_text("")
        self.server_label.set_visible(False)

    def reset(self, log_path=None):
        if self._drain_id:
            GLib.source_remove(self._drain_id)
            self._drain_id = 0
        self._pending.clear()
        self._discard_line = False
        self._log_identity = None
        self.current_log_path = log_path
        self.last_pos = 0
        self.line_count = 0
        self.buffer.set_text("")
        self.close_search()
        self.stop_server_location_polling()
        if log_path:
            self.status_label.set_text(log_path.name)
            self.start_server_location_polling()
        else:
            self.status_label.set_text("")

    def update(self):
        log_path = None
        session = self.window.session
        if session and session.log_path:
            log_path = session.log_path
        elif self.window.last_log:
            log_path = self.window.last_log

        if not log_path or not log_path.exists():
            self.stop_server_location_polling()
            return

        if self.current_log_path != log_path:
            self.reset(log_path)

        try:
            with open(log_path, "rb") as f:
                info = os.fstat(f.fileno())
                identity = (info.st_dev, info.st_ino)
                if (
                    self._log_identity is not None and self._log_identity != identity
                ) or info.st_size < self.last_pos:
                    self.reset(log_path)
                self._log_identity = identity
                f.seek(self.last_pos)
                chunk = f.read(max(0, self.READ_BYTES - len(self._pending)))
                self.last_pos = f.tell()
                self._pending.extend(chunk)
                eof = self.last_pos >= info.st_size
            inserted = False
            for _ in range(self.LINES_PER_UPDATE):
                newline = self._pending.find(b"\n")
                if self._discard_line:
                    if newline < 0:
                        self._pending.clear()
                        break
                    del self._pending[: newline + 1]
                    self._discard_line = False
                    continue
                shortened = newline > self.LINE_BYTES or (
                    newline < 0 and len(self._pending) > self.LINE_BYTES
                )
                if newline >= 0:
                    line = bytes(self._pending[: min(newline, self.LINE_BYTES)])
                    del self._pending[: newline + 1]
                elif shortened:
                    line = bytes(self._pending[: self.LINE_BYTES])
                    self._pending.clear()
                    self._discard_line = True
                elif eof and self.window.session is None and self._pending:
                    line = bytes(self._pending)
                    self._pending.clear()
                else:
                    break
                text = line.decode("utf-8", errors="replace")
                if shortened:
                    text += " … [open the log file for the full line]"
                # Joining line fragments before decoding preserves split UTF-8
                # characters and avoids inventing a new line for each write.
                self._insert_highlighted_text(text + "\n")
                inserted = True
            if inserted:
                self.status_label.set_text(f"{log_path.name} ({self.line_count} l.)")
                lines = self.buffer.get_line_count()
                if lines > 5000:
                    trim = self.buffer.get_start_iter()
                    trim.forward_lines(lines - 4000)
                    self.buffer.delete(self.buffer.get_start_iter(), trim)
                chars = self.buffer.get_char_count()
                if chars > self.VIEW_CHARS:
                    trim = self.buffer.get_iter_at_offset(
                        chars - self.VIEW_CHARS * 3 // 4
                    )
                    trim.forward_line()
                    self.buffer.delete(self.buffer.get_start_iter(), trim)
                if (
                    self.search_revealer.get_reveal_child()
                    and self.search_entry.get_text().strip()
                ):
                    # Offsets may have moved after trimming. Invalidate them
                    # immediately, then refresh once per bounded UI interval.
                    self.matches = []
                    self.current_match_idx = -1
                    self.buffer.remove_tag(
                        self.tag_current,
                        self.buffer.get_start_iter(),
                        self.buffer.get_end_iter(),
                    )
                    if not self._search_refresh_id:
                        self._search_refresh_id = GLib.timeout_add(
                            100, self._refresh_search
                        )
                if self.auto_scroll:
                    self.text_view.scroll_to_mark(self._end_mark, 0.0, False, 0.0, 1.0)
            backlog = (
                not eof
                or b"\n" in self._pending
                or len(self._pending) > self.LINE_BYTES
            )
            if backlog and not self._drain_id:
                self._drain_id = GLib.idle_add(self._drain_log)
        except Exception:
            pass

    def _drain_log(self):
        self._drain_id = 0
        if self.get_mapped():
            self.update()
        return False

    def _refresh_search(self):
        self._search_refresh_id = 0
        if self.search_revealer.get_reveal_child():
            query = self.search_entry.get_text().strip()
            self._rebuild_search(query, preserve=query == self._search_query)
        return False

    def _on_scroll_toggled(self, btn):
        self.auto_scroll = btn.get_active()
        if self.auto_scroll:
            self.text_view.scroll_to_mark(self._end_mark, 0.0, False, 0.0, 1.0)

    def _on_copy_clicked(self, _btn):
        raw = self.buffer.get_text(
            self.buffer.get_start_iter(), self.buffer.get_end_iter(), True
        )
        clean = re.sub(r"^\s*\d+\s*│\s*", "", raw, flags=re.MULTILINE)
        if clean:
            Gdk.Display.get_default().get_clipboard().set(clean)
            dialogs._toast(self.window.toasts, _("Logs copied to clipboard"))

    def _on_clear_clicked(self, _btn):
        if self._drain_id:
            GLib.source_remove(self._drain_id)
            self._drain_id = 0
        self._pending.clear()
        self._discard_line = False
        if self.current_log_path:
            try:
                self.last_pos = self.current_log_path.stat().st_size
            except OSError:
                pass
        self.buffer.set_text("")
        self.line_count = 0
        self.close_search()
