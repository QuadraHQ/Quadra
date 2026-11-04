"""The Settings page and its tabs."""

import os
import threading
from pathlib import Path

from gi.repository import Adw, Gio, GLib, Gtk

from .. import __version__, backends, core, dns, i18n
from ..darling import runtime, runtime_updates
from ..i18n import _
from ..rosemary import install as rosemary_install
from ..rosemary import runtime as rosemary_runtime
from ..rosemary import xr as rosemary_xr
from . import dialogs, flags_page, system, widgets

__all__ = [
    "DNS_CHOICES",
    "ROSEMARY_SETTINGS",
    "SettingsPage",
]

ROSEMARY_SETTINGS = (
    "runtime_backend",
    "rosemary_source",
    "rosemary_program",
    "rosemary_runners",
    "rosemary_prefix",
    "rosemary_pinned_build",
)

DNS_CHOICES = [
    ("system", "System (Darling default)"),
    ("quad9", "Quad9 (9.9.9.9, encrypted)"),
    ("cloudflare", "Cloudflare (1.1.1.1, encrypted)"),
    ("google", "Google (8.8.8.8, encrypted)"),
    ("custom", "Custom"),
]


class SettingsPage(Adw.Bin):
    def __init__(self, window):
        super().__init__()
        self.window = window
        self._checking_updates = False
        settings = window.settings

        toolbar_view = Adw.ToolbarView()
        self.stack = Adw.ViewStack()

        # 1. Environment page (Runtime, Interface, Darling or Rosemary, Game, DNS, Diagnostics)
        self.env_page = Adw.PreferencesPage()

        launcher_group = Adw.PreferencesGroup(title="Quadra")
        self.launcher_version_row = Adw.ActionRow(
            title=_("Launcher version"), subtitle=f"v{__version__}"
        )
        self.check_launcher_btn = Gtk.Button(
            label=_("Check for updates"), valign=Gtk.Align.CENTER
        )
        self._launcher_update_handler = self.check_launcher_btn.connect(
            "clicked", lambda *_args: self.check_launcher_update()
        )
        self.launcher_version_row.add_suffix(self.check_launcher_btn)
        launcher_group.add(self.launcher_version_row)

        force_update = widgets._button_row(_("Force update"))
        force_update.connect("activated", lambda *_args: self.force_update_launcher())
        launcher_group.add(force_update)

        self.launcher_progress = Gtk.ProgressBar(
            show_text=True,
            margin_top=6,
            margin_bottom=6,
            margin_start=12,
            margin_end=12,
            visible=False,
        )
        launcher_group.add(
            Gtk.ListBoxRow(
                activatable=False, selectable=False, child=self.launcher_progress
            )
        )
        self.env_page.add(launcher_group)

        runtime_group = Adw.PreferencesGroup(
            title=_("Runtime"),
            description=_(
                "Which layer runs the macOS Roblox client. Rosemary (microlift/rosemary) is Quadra's "
                "default: it runs an unpatched client, and Quadra installs Rosemary and Roblox for it "
                "when you press Play. Darling is outdated, but still has the most features (Quadra "
                "M2VK, web sign-in)."
            ),
        )
        rosemary_bundled = bool(backends.bundled_rosemary())
        self.backend_row = widgets.ComboRow(
            title=_("Run Roblox with"),
            model=Gtk.StringList.new(["Darling", "Rosemary"]),
            sensitive=not rosemary_bundled,
        )
        if rosemary_bundled:
            self.backend_row.set_subtitle(
                _(
                    "Nix manages Rosemary. Update or switch it through your Nix configuration."
                )
            )
        self.backend_row.set_selected(1 if window.rosemary_selected() else 0)
        self._backend_handler = self.backend_row.connect(
            "notify::selected", self._backend_selected
        )
        window.watch_setting("runtime_backend", self._show_backend)
        runtime_group.add(self.backend_row)
        self.env_page.add(runtime_group)

        interface = Adw.PreferencesGroup(title=_("Interface"))
        codes = list(i18n.LANGUAGES)
        language = widgets.ComboRow(
            title=_("Language"), model=Gtk.StringList.new(list(i18n.LANGUAGES.values()))
        )
        language.set_selected(codes.index(i18n.language()))
        language.connect(
            "notify::selected",
            lambda row, _pspec: window.set_language(codes[row.get_selected()]),
        )
        interface.add(language)
        self.appearance_codes = ("grid", "gnome", "monochrome")
        self.appearance_row = widgets.ComboRow(
            title=_("Launcher appearance"),
            subtitle=_("Applies immediately. GNOME follows your system appearance."),
            model=Gtk.StringList.new(
                [_("Grid"), _("GNOME (classic)"), _("Monochrome · aubree.lat")]
            ),
        )
        widgets.bind_combo(
            window,
            self.appearance_row,
            "launcher_appearance",
            self.appearance_codes,
            "gnome",
        )
        interface.add(self.appearance_row)
        tray_switch = Adw.SwitchRow(
            title=_("Tray icon"),
            subtitle=_(
                "Open settings or stop Roblox from the panel's tray while Quadra runs"
            ),
        )
        interface.add(widgets.bind_switch(window, tray_switch, "tray_icon", True))
        self.env_page.add(interface)

        self._runtime_codes = runtime.VARIANTS
        self._runtime_labels = {
            "regular": _("DarlingHQ's Darling"),
            "ciderdarling": _("QuadraHQ's CiderDarling"),
        }
        bundled = runtime.is_bundled()
        self._runtime_choices = self._installed_runtime_choices()
        darling_group = Adw.PreferencesGroup(
            title="Darling", description=_("Each runtime keeps its own Roblox sign-in.")
        )
        self.darling_group = darling_group
        labels = [
            self._runtime_labels[variant]
            if variant in self._runtime_choices or bundled
            else _("{runtime} (not installed)", runtime=self._runtime_labels[variant])
            for variant in self._runtime_codes
        ]
        self.runtime_row = widgets.ComboRow(
            title=_("Darling runtime"),
            model=Gtk.StringList.new(labels),
            sensitive=not bundled,
            use_subtitle=True,
        )
        active = core.DARLING_RUNTIME["runtime"]
        self.runtime_row.set_selected(self._runtime_codes.index(active))
        darling_group.add(self.runtime_row)
        self.runtime_active_row = Adw.ActionRow(
            title=_("Active runtime"), subtitle=self._runtime_labels[active]
        )
        self.runtime_apply_button = Gtk.Button(
            label=_("Apply and restart Quadra"),
            valign=Gtk.Align.CENTER,
            visible=not bundled,
        )
        self.runtime_apply_button.add_css_class("suggested-action")
        self.runtime_apply_button.connect(
            "clicked", lambda *_args: self.apply_runtime()
        )
        self.runtime_refresh_button = Gtk.Button(
            icon_name="view-refresh-symbolic",
            valign=Gtk.Align.CENTER,
            tooltip_text=_("Refresh installed runtimes"),
            visible=not bundled,
        )
        self.runtime_refresh_button.connect(
            "clicked",
            lambda *_args: (
                self.refresh_runtime_choices(),
                window.check_ciderdarling_updates(force=True),
            ),
        )
        self.runtime_active_row.add_suffix(self.runtime_refresh_button)
        self.runtime_active_row.add_suffix(self.runtime_apply_button)
        darling_group.add(self.runtime_active_row)
        self.runtime_install_row = Adw.ActionRow(
            title=_("Install CiderDarling"),
            subtitle=_("Opens a separate terminal for the build and sudo prompts."),
            visible=False,
        )
        self.runtime_install_button = Gtk.Button(
            label=_("Install in terminal"), valign=Gtk.Align.CENTER
        )
        self.runtime_install_button.add_css_class("suggested-action")
        self.runtime_install_button.connect(
            "clicked", lambda *_args: window.install_ciderdarling()
        )
        self.runtime_install_row.add_suffix(self.runtime_install_button)
        darling_group.add(self.runtime_install_row)
        self.cider_settings_row = Adw.ActionRow(
            title=_("CiderDarling settings"),
            subtitle=_("Graphics card, watchdog, tracing and diagnostics"),
            activatable=True,
            visible=False,
        )
        self.cider_settings_row.add_suffix(Gtk.Image(icon_name="go-next-symbolic"))
        self.cider_settings_row.connect(
            "activated", lambda *_args: self.set_tab("ciderdarling")
        )
        darling_group.add(self.cider_settings_row)
        self._runtime_handler = self.runtime_row.connect(
            "notify::selected", lambda *_args: self._runtime_selection_changed()
        )
        self._runtime_selection_changed()

        window.runtime_only(darling_group, "darling", hide=True)
        self.env_page.add(darling_group)
        self._build_rosemary_groups(settings)

        game = Adw.PreferencesGroup(title=_("Game"))
        # The renderer and graphics card rows also appear on the CiderDarling
        # tab; every copy shows and saves the same setting.
        self._renderer_rows = []
        renderer_row = self._renderer_row()
        game.add(renderer_row)
        window.runtime_only(renderer_row, "renderer", hide=True)

        self._build_m2vk_group(settings)

        self._gpu_codes = ["auto"]
        self._gpu_rows = []
        self.gpu_row = self._gpu_row()
        game.add(self.gpu_row)

        backend_codes = ["x11", "wayland"]
        backend = widgets.ComboRow(
            title=_("Window backend"),
            subtitle=_("Applies on next launch."),
            model=Gtk.StringList.new([_("X11 / Xwayland"), _("Wayland")]),
        )
        widgets.bind_combo(window, backend, "display_backend", backend_codes, "x11")
        game.add(backend)
        window.runtime_only(backend, "input", hide=True)

        from ..display import validated_dpi_scale

        ui_scale = Adw.SpinRow.new_with_range(100, 400, 25)
        ui_scale.set_digits(0)
        ui_scale.set_title(_("Roblox UI scale"))
        ui_scale.set_subtitle(
            _("100–400%. Keeps the display resolution. Applies on next launch.")
        )
        widgets.bind_spin(
            window,
            ui_scale,
            "dpi_scale",
            1.0,
            to_setting=lambda value: validated_dpi_scale(value / 100),
            to_row=lambda value: validated_dpi_scale(value) * 100,
        )
        game.add(ui_scale)
        window.runtime_only(ui_scale, "input", hide=True)

        mangohud = Adw.SwitchRow(
            title=_("MangoHud overlay"),
            subtitle=_(
                "Show FPS, frametimes and CPU/GPU usage. Requires MangoHud; applies on next launch."
            ),
        )
        game.add(widgets.bind_switch(window, mangohud, "mangohud"))

        sensitivity = Adw.SpinRow.new_with_range(0.1, 5.0, 0.05)
        sensitivity.set_digits(2)
        sensitivity.set_title(_("Camera sensitivity"))
        sensitivity.set_subtitle(
            _("Mouse movement multiplier while rotating the camera")
        )
        widgets.bind_spin(
            window,
            sensitivity,
            "mouse_sensitivity",
            1.0,
            to_setting=lambda value: round(value, 2),
        )
        game.add(sensitivity)
        window.runtime_only(sensitivity, "input", hide=True)

        scroll_sens = Adw.SpinRow.new_with_range(0.5, 10.0, 0.5)
        scroll_sens.set_digits(1)
        scroll_sens.set_title(_("Scroll sensitivity"))
        scroll_sens.set_subtitle(_("Mouse wheel scroll speed in menus and interface"))
        widgets.bind_spin(
            window,
            scroll_sens,
            "scroll_sensitivity",
            1.5,
            to_setting=lambda value: round(value, 2),
        )
        game.add(scroll_sens)
        window.runtime_only(scroll_sens, "input", hide=True)

        raw_mouse = Adw.SwitchRow(
            title=_("Raw mouse input"),
            subtitle=_(
                "Camera moves by the mouse's own motion, without pointer acceleration (XInput 2)"
            ),
        )
        game.add(widgets.bind_switch(window, raw_mouse, "raw_mouse", True))
        window.runtime_only(raw_mouse, "input", hide=True)

        menu_bar = Adw.SwitchRow(
            title=_("Hide the macOS menu bar"),
            subtitle=_("The Roblox, Edit, Window… strip at the top of the game window"),
        )
        game.add(widgets.bind_switch(window, menu_bar, "hide_menu_bar"))
        window.runtime_only(menu_bar, "menu_bar")

        hide_launcher = Adw.SwitchRow(
            title=_("Hide launcher while playing"),
            subtitle=_("Hide the launcher window while the game is running"),
        )
        game.add(
            widgets.bind_switch(window, hide_launcher, "hide_launcher_on_launch", True)
        )

        reopen = Adw.SwitchRow(
            title=_("Open settings after Roblox exits"),
            subtitle=_("Otherwise Quadra closes with the game"),
        )
        game.add(widgets.bind_switch(window, reopen, "open_settings_after_exit"))

        playtime_switch = Adw.SwitchRow(
            title=_("Show playtime"),
            subtitle=_("Show accumulated playtime on the Play page"),
        )
        game.add(widgets.bind_switch(window, playtime_switch, "show_playtime", True))

        server_location = Adw.SwitchRow(
            title=_("Show server location"),
            subtitle=_(
                "Sends the game server's IP address to RoValra (apis.rovalra.com) while you play"
            ),
        )
        game.add(widgets.bind_switch(window, server_location, "server_location_lookup"))

        server_notify = Adw.SwitchRow(
            title=_("Notify of server location"),
            subtitle=_(
                "Show a desktop notification with the server's location when you join one"
            ),
        )
        game.add(
            widgets.bind_switch(window, server_notify, "server_location_notify", True)
        )

        cores = os.cpu_count() or 1
        cpu_limit = Adw.SpinRow.new_with_range(0, cores, 1)
        cpu_limit.set_title(_("Limit CPU cores (experimental)"))
        cpu_limit.set_subtitle(
            _("0 uses all cores. Applied a few seconds after the game starts.")
        )
        widgets.bind_spin(window, cpu_limit, "roblox_cpu_limit", 0, to_setting=int)
        game.add(cpu_limit)

        self.env_page.add(game)

        # Quadra M2VK has a tab of its own: the renderer and graphics card to choose it with, then its settings.
        self.m2vk_page = Adw.PreferencesPage()
        graphics_group = Adw.PreferencesGroup(title=_("Renderer"))
        graphics_group.add(self._renderer_row())
        graphics_group.add(self._gpu_row(always_visible=True))
        self.m2vk_page.add(graphics_group)
        self.m2vk_page.add(self.m2vk_group)

        # VR headsets through the host's OpenXR runtime (the VR tab)
        from .. import vr

        vr_group = Adw.PreferencesGroup(
            title=_("VR headset"),
            description=_(
                "Experimental. Plays in Roblox's own VR mode on a headset connected through an OpenXR "
                "runtime such as WiVRn (Meta Quest) or Monado. Applies on next launch."
            ),
        )
        _runtime, vr_status = vr.status()
        vr_switch = Adw.SwitchRow(
            title=_("VR headset (OpenXR)"),
            subtitle=vr_status,
            active=settings.get("vr", False),
        )
        vr_scale = Adw.SpinRow.new_with_range(40, 150, 5)
        vr_scale.set_digits(0)
        vr_scale.set_title(_("VR render resolution"))
        vr_scale.set_subtitle(
            _("Percent of the headset's recommended size per eye. Lower is smoother.")
        )
        widgets.bind_spin(
            window,
            vr_scale,
            "vr_render_scale",
            vr.DEFAULT_RENDER_SCALE,
            to_setting=vr.validated_render_scale,
            to_row=vr.validated_render_scale,
        )
        vr_mirror = Adw.SwitchRow(
            title=_("Show the headset view in the game window"),
            subtitle=_(
                "Both eyes side by side on the desktop. Off saves a little time per frame."
            ),
        )
        widgets.bind_switch(window, vr_mirror, "vr_mirror", True)

        def vr_toggled(row, _pspec=None):
            vr_scale.set_sensitive(row.get_active())
            vr_mirror.set_sensitive(row.get_active())

        confirmed = [False]

        def vr_changed(row, _pspec=None):
            # Turning VR on asks first (unless "Don't show this again" was kept): the switch goes back to off
            # and only the dialog's go-ahead turns it on, so VR is never saved as on without the warning.
            if (
                row.get_active()
                and not confirmed[0]
                and not window.settings.get("vr_warning_hidden", False)
            ):
                row.set_active(False)

                def accepted():
                    confirmed[0] = True
                    row.set_active(True)
                    confirmed[0] = False

                dialogs._vr_warning_dialog(window, accepted)
                return
            window.set_setting("vr", row.get_active())
            vr_toggled(row)

        vr_switch.connect("notify::active", vr_changed)
        vr_toggled(vr_switch)
        for row in (vr_switch, vr_scale, vr_mirror):
            vr_group.add(row)
        # Its own tab, added after Roblox below.
        self.vr_page = Adw.PreferencesPage()
        self.vr_page.add(vr_group)
        window.runtime_only(self.vr_page, "vr")

        # 3. Discord Rich Presence
        discord_group = Adw.PreferencesGroup(title=_("Discord Rich Presence"))

        discord_rpc = Adw.SwitchRow(
            title=_("Enable Discord Rich Presence"),
            subtitle=_("Show current game and playtime in your Discord status"),
        )
        discord_group.add(widgets.bind_switch(window, discord_rpc, "discord_rpc", True))
        discord_options = []
        for key, title, subtitle, default in (
            (
                "discord_rpc_game",
                _("Show experience name in Discord"),
                _("Display the title and creator of the place you are playing"),
                True,
            ),
            (
                "discord_rpc_icon",
                _("Show experience thumbnail in Discord"),
                _("Replace the Quadra icon with the game's icon"),
                False,
            ),
            (
                "discord_rpc_time",
                _("Show elapsed time in Discord"),
                _("Display how long you have been playing in your status"),
                True,
            ),
        ):
            option = Adw.SwitchRow(title=title, subtitle=subtitle)
            discord_group.add(widgets.bind_switch(window, option, key, default))
            discord_options.append(option)

        def show_discord_options(_key, enabled):
            for option in discord_options:
                option.set_sensitive(enabled)

        show_discord_options("discord_rpc", settings.get("discord_rpc", True))
        window.watch_setting("discord_rpc", show_discord_options)

        self.env_page.add(discord_group)

        dns_group = Adw.PreferencesGroup(
            title=_("DNS for Roblox"),
            description=_(
                "Only Roblox uses this server, the rest of the system keeps its own DNS. "
                "Helps when some Roblox images or servers do not load."
            ),
        )
        dns_codes = [code for code, _label in DNS_CHOICES]
        server = widgets.ComboRow(
            title=_("DNS server"),
            model=Gtk.StringList.new([_(label) for _code, label in DNS_CHOICES]),
        )
        widgets.bind_combo(window, server, "dns", dns_codes, "system")
        custom = Adw.EntryRow(title=_("Custom server"))
        custom.set_text(settings.get("dns_custom", ""))
        custom.set_show_apply_button(True)
        custom.set_tooltip_text(
            _("IP address, optionally with :port. Plain DNS, not encrypted.")
        )
        custom.set_visible(settings.get("dns", "system") == "custom")
        window.watch_setting(
            "dns", lambda _key, code: custom.set_visible(code == "custom")
        )

        def custom_applied(row):
            text = row.get_text().strip()
            try:
                dns.parse_server(text)
            except ValueError as error:
                row.add_css_class("error")
                dialogs._toast(window.toasts, str(error))
                return
            row.remove_css_class("error")
            window.set_setting("dns_custom", text)

        custom.connect("apply", custom_applied)
        dns_group.add(server)
        dns_group.add(custom)
        block = Adw.SwitchRow(
            title=_("Block Roblox telemetry"),
            subtitle=_(
                'Answers Roblox\'s telemetry and crash-upload hosts with "not found". '
                "Needs a DNS server above other than the system's."
            ),
        )
        dns_group.add(widgets.bind_switch(window, block, "block_telemetry"))
        self.env_page.add(dns_group)
        window.runtime_only(dns_group, "dns", hide=True)

        diagnostics = Adw.PreferencesGroup(
            title=_("Diagnostics"),
            description=_(
                "Detailed logs for debugging. They slow the game down, enable only when needed."
            ),
        )
        for key, title in [
            ("diagnostic_signals", "Crash diagnostics"),
            ("trace_udp", "Network tracing (UDP)"),
            ("trace_lock", "Mouse lock tracing"),
            ("trace_events", "Mouse event tracing"),
            ("trace_gl", "OpenGL tracing"),
            ("trace_keys", "Keyboard tracing"),
            ("fps_log", "Frame rate in the log"),
        ]:
            row = Adw.SwitchRow(title=_(title), subtitle=core.TRACE_ENV[key])
            diagnostics.add(widgets.bind_switch(window, row, key))
        diagnostics.add(self._allow_debugger_row())
        collect = widgets._button_row(_("Collect diagnostics"))
        collect.connect("activated", lambda *_args: window.collect_diagnostics())
        diagnostics.add(collect)
        hardware = widgets._button_row(_("Collect hardware report"))
        hardware.set_tooltip_text(
            _(
                "Tests GPU rendering, MSAA, CPU limits and window resources. Saves a local report."
            )
        )
        hardware.connect("activated", lambda *_args: window.collect_hardware_report())
        diagnostics.add(hardware)
        logs = widgets._button_row(_("Open logs folder"))
        logs.connect("activated", lambda *_args: self.open_logs())
        diagnostics.add(logs)
        rebuild = widgets._button_row(_("Rebuild shim"))
        rebuild.connect("activated", lambda *_args: self.rebuild())
        diagnostics.add(rebuild)
        restart = widgets._button_row(_("Restart Darling"))
        restart.connect("activated", lambda *_args: self.restart_darling())
        diagnostics.add(restart)
        self.env_page.add(diagnostics)
        window.runtime_only(diagnostics, "diagnostics", hide=True)

        self.stack.add_titled_with_icon(
            self.env_page, "env", _("Environment"), "preferences-system-symbolic"
        )
        m2vk_tab = self.stack.add_titled_with_icon(
            self.m2vk_page, "m2vk", "Quadra M2VK", "video-display-symbolic"
        )
        window.runtime_only(m2vk_tab, "m2vk", hide=True)

        # 2. Roblox page (Download, updates, version, account)
        self.roblox_page = Adw.PreferencesPage()

        roblox = Adw.PreferencesGroup(title="Roblox")
        self.version_row = Adw.ActionRow(
            title=_("Installed version"),
            subtitle=core.installed_version() or _("not found"),
        )
        self.update_button = Gtk.Button(
            label=_("Check for updates"), valign=Gtk.Align.CENTER
        )
        self._update_handler = self.update_button.connect(
            "clicked", lambda *_args: self.check_updates()
        )
        self.version_row.add_suffix(self.update_button)
        roblox.add(self.version_row)
        self.channel_row = Adw.ActionRow(title=_("Last client channel"))
        self._show_channel("roblox_channel", settings.get("roblox_channel"))
        window.watch_setting("roblox_channel", self._show_channel)
        roblox.add(self.channel_row)
        window.watch_client(self._show_installed_client)

        auto_update = Adw.SwitchRow(
            title=_("Check for Roblox updates on startup"),
            subtitle=_("Prompt to update if a newer version of Roblox is available"),
        )
        roblox.add(
            widgets.bind_switch(window, auto_update, "auto_check_roblox_updates", True)
        )

        self.progress = Gtk.ProgressBar(
            show_text=True,
            margin_top=6,
            margin_bottom=6,
            margin_start=12,
            margin_end=12,
            visible=False,
        )
        progress_row = Gtk.ListBoxRow(
            activatable=False, selectable=False, child=self.progress
        )
        roblox.add(progress_row)

        delete_roblox = widgets._button_row(_("Delete Roblox"))
        delete_roblox.add_css_class("destructive-action")
        delete_roblox.connect("activated", lambda *_args: self.delete_roblox())
        roblox.add(delete_roblox)
        self.roblox_page.add(roblox)
        window.runtime_only(roblox, "client_updates", hide=True)
        rosemary_roblox = self._build_rosemary_client_group()
        self.roblox_page.add(rosemary_roblox)
        window.runtime_only(rosemary_roblox, "rosemary", hide=True)
        self.refresh_rosemary()
        window.watch_setting(
            ROSEMARY_SETTINGS, lambda _key, _value: self.refresh_rosemary()
        )

        # The startup render throttle patch gets a group of its own: the
        # button applies or reverts it by hand, the switch decides whether
        # launches and updates keep it applied. Installing turns the switch
        # on and removing turns it off, so the two cannot contradict.
        throttle = Adw.PreferencesGroup(title=_("Throttle patch"))
        self.throttle_row = widgets._button_row(_("Throttle patch"))
        self.throttle_row.set_sensitive(False)
        self.throttle_row.connect(
            "activated", lambda *_args: self.toggle_throttle_patch()
        )
        throttle.add(self.throttle_row)

        auto_patch = Adw.SwitchRow(
            title=_("Apply the throttle patch automatically"),
            subtitle=_("Patch the client on every launch and after Roblox updates"),
        )
        throttle.add(
            widgets.bind_switch(window, auto_patch, "auto_patch_throttle", True)
        )
        self._in_thread(core.throttle_patch_state, self._throttle_state_done)
        self.roblox_page.add(throttle)
        window.runtime_only(throttle, "throttle")

        account = Adw.PreferencesGroup(title=_("Account"))
        logout = widgets._button_row(_("Sign out"))
        logout.add_css_class("destructive-action")
        logout.connect("activated", lambda *_args: self.logout())
        account.add(logout)
        self.roblox_page.add(account)

        self.stack.add_titled_with_icon(
            self.roblox_page, "roblox", "Roblox", "application-x-executable-symbolic"
        )

        self.stack.add_titled_with_icon(self.vr_page, "vr", "VR", "quadra-vr-symbolic")

        # 3. Fast flags page
        self.flags_page = flags_page.FlagsPage(window)
        self.stack.add_titled_with_icon(
            self.flags_page, "flags", _("Fast flags"), "preferences-other-symbolic"
        )

        # 4. CiderDarling page, shown while CiderDarling is installed
        self.cider_page = self._ciderdarling_page()
        window.runtime_only(self.cider_page, "cider")
        self.cider_stack_page = self.stack.add_titled_with_icon(
            self.cider_page,
            "ciderdarling",
            "CiderDarling",
            "applications-engineering-symbolic",
        )
        self.refresh_ciderdarling()

        self.appearance_tabs = widgets._add_tabs(toolbar_view, self.stack)

        self.set_child(toolbar_view)

    def set_tab(self, tab):
        self.stack.set_visible_child_name(tab)

    def flush(self):
        self.flags_page.flush()

    def _show_channel(self, _key, channel):
        self.channel_row.set_subtitle(channel or _("Release (default)"))

    def _show_installed_client(self):
        self.version_row.set_subtitle(core.installed_version() or _("not found"))
        self.refresh_rosemary()

    def selected_runtime(self):
        index = self.runtime_row.get_selected()
        return self._runtime_codes[index] if index < len(self._runtime_codes) else None

    def select_runtime(self, variant):
        self.runtime_row.set_selected(self._runtime_codes.index(variant))

    def installed_runtime(self, variant):
        return self._runtime_choices.get(variant)

    def runtime_installed(self, variant):
        return variant in self._runtime_choices

    def ciderdarling_revisions(self):
        """(installed, pinned) CiderDarling source revisions; None when unknown."""
        choice = self._runtime_choices.get("ciderdarling")
        installed = (
            runtime.metadata(choice["sysroot"]).get("source_revision")
            if choice
            else None
        )
        return (
            installed if isinstance(installed, str) else None,
            (self.window.cider_pin or {}).get("revision"),
        )

    def _installed_runtime_choices(self):
        choices = runtime.installed_runtimes(core.DARLING_RUNTIME, pinned=None)
        release, choice = self.window.cider_release, choices.get("ciderdarling")
        if release and choice:
            metadata = runtime.metadata(choice["sysroot"])
            if metadata.get("source_revision") == release[
                "source_revision"
            ] or runtime_updates.newer(release, metadata):
                return runtime.installed_runtimes(
                    core.DARLING_RUNTIME, pinned=release["source_revision"]
                )
        return choices

    def ciderdarling_update_needed(self):
        """An available, published descendant of the installed runtime."""
        choice = self._runtime_choices.get("ciderdarling")
        release = self.window.cider_release
        return bool(
            choice
            and release
            and not runtime.is_bundled()
            and runtime_updates.newer(release, runtime.metadata(choice["sysroot"]))
        )

    def _runtime_selection_changed(self):
        selected = self.selected_runtime()
        choice = self._runtime_choices.get(selected)
        bundled = runtime.is_bundled()
        installing = self.window.runtime_install_job is not None
        update = self.ciderdarling_update_needed()
        changed = choice is not None and any(
            choice[key] != core.DARLING_RUNTIME[key] for key in runtime.SELECTION_KEYS
        )
        self.runtime_apply_button.set_sensitive(
            changed and not bundled and not installing
        )
        self.runtime_install_row.set_visible(
            not bundled
            and (
                installing or update or (selected == "ciderdarling" and choice is None)
            )
        )
        self.runtime_install_button.set_sensitive(not bundled and not installing)
        self.runtime_install_row.set_title(
            _("Update CiderDarling") if update else _("Install CiderDarling")
        )
        self.runtime_install_button.set_label(
            _("Update in terminal") if update else _("Install in terminal")
        )
        if installing:
            self.runtime_install_row.set_subtitle(
                _("Installing CiderDarling in another terminal…")
            )
        elif update:
            installed, pinned = self.ciderdarling_revisions()
            self.runtime_install_row.set_subtitle(
                _(
                    "Installed {installed}, update {pinned} is available. Opens a separate terminal for installation.",
                    installed=installed[:12],
                    pinned=pinned[:12],
                )
            )
        else:
            self.runtime_install_row.set_subtitle(
                _("Opens a separate terminal for the build and sudo prompts.")
            )
        if os.environ.get("QUADRA_BUNDLED_DARLING"):
            subtitle = _(
                "Nix manages this Darling runtime. Update or switch it through your Nix configuration."
            )
        elif bundled:
            subtitle = _("Flatpak uses its bundled Darling runtime.")
        elif installing:
            subtitle = _(
                "Installing in a separate terminal. Your current runtime remains selected."
            )
        elif selected == "ciderdarling" and choice is None:
            subtitle = _(
                "Install CiderDarling in a separate terminal, then apply the switch."
            )
        elif choice is None:
            subtitle = _(
                "Install this runtime with Quadra's installer before switching."
            )
        else:
            subtitle = _("Each runtime keeps its own Roblox sign-in.")
        self.darling_group.set_description(subtitle)

    def refresh_runtime_choices(self):
        try:
            choices = self._installed_runtime_choices()
        except (OSError, RuntimeError, ValueError) as error:
            dialogs._error_dialog(
                self.window, _("Could not inspect installed runtimes"), str(error)
            )
            return
        selected = self.runtime_row.get_selected()
        labels = [
            self._runtime_labels[variant]
            if variant in choices or runtime.is_bundled()
            else _("{runtime} (not installed)", runtime=self._runtime_labels[variant])
            for variant in self._runtime_codes
        ]
        self._runtime_choices = choices
        self.runtime_row.handler_block(self._runtime_handler)
        try:
            self.runtime_row.set_model(Gtk.StringList.new(labels))
            self.runtime_row.set_selected(selected)
        finally:
            self.runtime_row.handler_unblock(self._runtime_handler)
        self._runtime_selection_changed()
        self.refresh_ciderdarling()

    def apply_runtime(self):
        if runtime.is_bundled() or not self.runtime_apply_button.get_sensitive():
            return
        if not self.window.begin("switching runtime"):
            return
        selected = self.selected_runtime()
        saved = False
        try:
            # Also protect Roblox launched outside this window but using the
            # same prefix. Never shut down another Darling installation.
            if core.roblox_pids():
                self.window.end(resume_pending=False)
                dialogs._toast(self.window.toasts, _("Close Roblox first"))
                return
            choice = self._installed_runtime_choices().get(selected)
            if choice is None:
                self.refresh_runtime_choices()
                raise RuntimeError(
                    _("Install this runtime with Quadra's installer before switching.")
                )
            self.flush()
            self.window.settings.save()
            runtime.save_selection(choice, current=core.DARLING_RUNTIME)
            saved = True
            self.window._stop_web()
            self.window._stop_rpc()
            system._restart_launcher()
        except (OSError, RuntimeError, ValueError) as error:
            self.window.end(resume_pending=False)
            details = str(error)
            if saved:
                details = (
                    _("Runtime saved. Close and reopen Quadra to apply it.")
                    + "\n\n"
                    + details
                )
            dialogs._error_dialog(
                self.window, _("Could not switch Darling runtime"), details
            )

    def rollback_runtime(self):
        if runtime.is_bundled() or not self.window.begin("switching runtime"):
            return
        saved = False
        try:
            if core.roblox_pids():
                dialogs._toast(self.window.toasts, _("Close Roblox first"))
                return
            choice = runtime.rollback_candidate(core.DARLING_RUNTIME)
            if choice is None:
                raise RuntimeError(
                    _("The previous CiderDarling installation is unavailable.")
                )
            self.flush()
            self.window.settings.save()
            runtime.save_selection(choice, current=core.DARLING_RUNTIME)
            saved = True
            self.window._stop_web()
            self.window._stop_rpc()
            system._restart_launcher()
        except (OSError, RuntimeError, ValueError) as error:
            details = str(error)
            if saved:
                details = (
                    _("Runtime saved. Close and reopen Quadra to apply it.")
                    + "\n\n"
                    + details
                )
            dialogs._error_dialog(
                self.window, _("Could not switch Darling runtime"), details
            )
        finally:
            self.window.end(resume_pending=False)

    def _show_backend(self, _key, backend):
        with self.backend_row.handler_block(self._backend_handler):
            self.backend_row.set_selected(1 if backend == "rosemary" else 0)
        self._show_gpu_rows()

    def _backend_selected(self, row, _pspec):
        window = self.window
        window.set_setting(
            "runtime_backend", "rosemary" if row.get_selected() == 1 else "darling"
        )
        if (
            window.rosemary_selected()
            and not window.rosemary_ready()
            and not window.busy
            and not window.setup_active
        ):
            self._install_rosemary_client()

    def _build_rosemary_groups(self, settings):
        window = self.window
        program = Adw.PreferencesGroup(title=_("Rosemary"))
        self._rosemary_sources = ["quadra", "upstream"]
        self.rosemary_source_row = widgets.ComboRow(
            title=_("Rosemary build"),
            subtitle=_(
                "Quadra's build adds what VR needs; the regular one is microlift's. Quadra installs the one you pick."
            ),
            model=Gtk.StringList.new(
                [_("Quadra's Rosemary (adds VR)"), _("Regular Rosemary")]
            ),
        )
        self.rosemary_source_row.set_selected(
            self._rosemary_sources.index(
                rosemary_install.normalize_source(settings.get("rosemary_source"))
            )
        )
        self._rosemary_source_handler = self.rosemary_source_row.connect(
            "notify::selected", self._rosemary_source_selected
        )
        window.watch_setting("rosemary_source", self._show_rosemary_source)
        program.add(self.rosemary_source_row)
        rosemary_bundled = bool(backends.bundled_rosemary())
        self.rosemary_runner_row = Adw.ComboRow(
            title=_("Rosemary runner"), use_subtitle=False
        )
        self._rosemary_runners = []
        self._runner_selected_handler = self.rosemary_runner_row.connect(
            "notify::selected", self._rosemary_runner_selected
        )
        self.rosemary_remove_runner_button = Gtk.Button(
            icon_name="user-trash-symbolic",
            tooltip_text=_("Remove from the list"),
            valign=Gtk.Align.CENTER,
            css_classes=["flat"],
        )
        self.rosemary_remove_runner_button.connect(
            "clicked", lambda *_args: self._remove_rosemary_runner()
        )
        self.rosemary_runner_row.add_suffix(self.rosemary_remove_runner_button)
        self.rosemary_install_button = Gtk.Button(
            label=_("Install"),
            valign=Gtk.Align.CENTER,
            css_classes=["suggested-action"],
        )
        self.rosemary_install_button.connect(
            "clicked", lambda *_args: self._install_rosemary()
        )
        self.rosemary_runner_row.add_suffix(self.rosemary_install_button)
        program.add(self.rosemary_runner_row)
        self._keep_typed_rosemary_program_listed()
        add_runner = Adw.ActionRow(
            title=_("Add your own Rosemary"),
            subtitle=_("Pick a rosemary program you built or downloaded"),
        )
        browse = Gtk.Button(label=_("Browse…"), valign=Gtk.Align.CENTER)
        browse.connect("clicked", lambda *_args: self._choose_rosemary_runner())
        add_runner.add_suffix(browse)
        add_runner.set_activatable_widget(browse)
        program.add(add_runner)
        if rosemary_bundled:
            program.set_description(
                _(
                    "Nix manages Rosemary. Update or switch it through your Nix configuration."
                )
            )
            for row in (
                self.rosemary_source_row,
                self.rosemary_install_button,
                add_runner,
            ):
                row.set_visible(False)
            self.rosemary_runner_row.set_sensitive(False)
        prefix = Adw.EntryRow(title=_("Prefix folder (empty: Quadra's own)"))
        prefix.set_text(settings.get("rosemary_prefix", ""))
        prefix.set_show_apply_button(True)
        prefix.connect(
            "apply",
            lambda row: window.set_setting("rosemary_prefix", row.get_text().strip()),
        )
        program.add(prefix)
        program_progress = Gtk.ProgressBar(show_text=True, visible=False, margin_top=6)
        program.add(program_progress)
        self.rosemary_progress_bars = [program_progress]
        self.env_page.add(program)

        options = Adw.PreferencesGroup(title=_("Options"))
        engine = widgets.ComboRow(
            title=_("Execution"),
            subtitle=_(
                "Native runs the machine's own code; emulated recompiles x86-64"
            ),
            model=Gtk.StringList.new([_("Automatic"), _("Native"), _("Emulated")]),
        )
        widgets.bind_combo(
            window, engine, "rosemary_engine", rosemary_runtime.ENGINES, "auto"
        )
        options.add(engine)
        opengl = Adw.SwitchRow(
            title=_("Force OpenGL"),
            subtitle=_("For machines without a Vulkan driver"),
        )
        options.add(widgets.bind_switch(window, opengl, "rosemary_force_opengl"))
        display = widgets.ComboRow(
            title=_("Display server"),
            model=Gtk.StringList.new([_("Default"), "Wayland", "X11 (XWayland)"]),
        )
        widgets.bind_combo(
            window, display, "rosemary_video_driver", rosemary_runtime.VIDEO_DRIVERS, ""
        )
        options.add(display)
        vsync = widgets.ComboRow(
            title="VSync",
            model=Gtk.StringList.new([_("Default"), _("Off"), _("On"), _("Adaptive")]),
        )
        widgets.bind_combo(
            window, vsync, "rosemary_swap_interval", rosemary_runtime.SWAP_INTERVALS, ""
        )
        options.add(vsync)
        present = widgets.ComboRow(
            title=_("Presentation mode"),
            subtitle=_("How frames reach the screen (Vulkan present mode)"),
            model=Gtk.StringList.new(
                [_("Default"), _("Immediate"), "FIFO", _("FIFO relaxed"), "Mailbox"]
            ),
        )
        widgets.bind_combo(
            window,
            present,
            "rosemary_presentation_mode",
            rosemary_runtime.PRESENTATION_MODES,
            "",
        )
        options.add(present)
        rate = Adw.EntryRow(
            title=_("Refresh rate told to the game (Hz, empty: the display's)")
        )
        rate.set_text(settings.get("rosemary_refresh_rate", ""))
        rate.set_show_apply_button(True)

        def rate_applied(row):
            text = row.get_text().strip()
            if text and not rosemary_runtime.refresh_rate(text):
                row.add_css_class("error")
                dialogs._toast(window.toasts, _("Enter a number of hertz above zero"))
                return
            row.remove_css_class("error")
            window.set_setting("rosemary_refresh_rate", text)

        rate.connect("apply", rate_applied)
        options.add(rate)
        title_bar = widgets.ComboRow(
            title=_("Title bar"),
            subtitle=_("Who draws the window's title bar"),
            model=Gtk.StringList.new(
                [_("Default"), _("Rosemary (Mac style)"), _("Window manager")]
            ),
        )
        widgets.bind_combo(
            window, title_bar, "rosemary_decorations", rosemary_runtime.DECORATIONS, ""
        )
        options.add(title_bar)
        menu = widgets.ComboRow(
            title=_("Menu placement"),
            subtitle=_('"Hide the macOS menu bar" in Game settings overrides this'),
            model=Gtk.StringList.new(
                [_("Default"), _("In the title bar"), _("Strip under it"), _("None")]
            ),
        )
        widgets.bind_combo(window, menu, "rosemary_menu", rosemary_runtime.MENUS, "")
        options.add(menu)
        for key, title, subtitle in (
            ("rosemary_confine_pointer", _("Keep the pointer inside the window"), None),
            (
                "rosemary_control_edit_keys",
                _("Control works as Command in editing shortcuts"),
                None,
            ),
            (
                "rosemary_plain_ctrl_click",
                _("Control-click is a plain left click"),
                _("Crouching with Control would otherwise be a right click"),
            ),
        ):
            row = Adw.SwitchRow(title=title, subtitle=subtitle or "")
            options.add(widgets.bind_switch(window, row, key))
        zink = Adw.SwitchRow(
            title=_("VR: OpenGL through Vulkan (zink)"),
            subtitle=_(
                "Roblox's VR needs its OpenGL renderer; this runs it on Vulkan with Mesa's zink"
            ),
        )
        options.add(widgets.bind_switch(window, zink, "rosemary_vr_zink"))
        server = Adw.SwitchRow(
            title=_("Process server"),
            subtitle=_("Stopped with the game; turn off to leave it running"),
        )
        options.add(
            widgets.bind_switch(window, server, "rosemary_process_server", True)
        )
        self.env_page.add(options)
        for group in (program, options):
            window.runtime_only(group, "rosemary", hide=True)

    def _build_rosemary_client_group(self):
        client_group = Adw.PreferencesGroup(title=_("Roblox for Rosemary"))
        self.rosemary_client_row = Adw.ActionRow(title=_("Installed version"))
        self.rosemary_client_button = Gtk.Button(
            label=_("Install or update"), valign=Gtk.Align.CENTER
        )
        self.rosemary_client_button.connect(
            "clicked", lambda *_args: self._install_rosemary_client()
        )
        self.rosemary_client_row.add_suffix(self.rosemary_client_button)
        client_group.add(self.rosemary_client_row)
        self.rosemary_rollback_row = Adw.ActionRow(title=_("Previous version"))
        self.rosemary_rollback_button = Gtk.Button(valign=Gtk.Align.CENTER)
        self.rosemary_rollback_button.connect(
            "clicked", lambda *_args: self._toggle_rosemary_rollback()
        )
        self.rosemary_rollback_row.add_suffix(self.rosemary_rollback_button)
        client_group.add(self.rosemary_rollback_row)
        client_progress = Gtk.ProgressBar(show_text=True, visible=False, margin_top=6)
        client_group.add(client_progress)
        self.rosemary_progress_bars.append(client_progress)

        self.rosemary_vr_row = Adw.ActionRow(title=_("VR support"))
        client_group.add(self.rosemary_vr_row)

        clear_cache = widgets._button_row(_("Clear Roblox cache"))
        clear_cache.set_tooltip_text(
            _(
                "Deletes the downloaded interface updates and assets from the prefix. "
                "Your sign-in is kept. The next launch downloads them again."
            )
        )
        clear_cache.connect("activated", lambda *_args: self.clear_roblox_cache())
        client_group.add(clear_cache)
        cache_on_launch = Adw.SwitchRow(
            title=_("Clear the cache before each launch"),
            subtitle=_(
                "Runs Roblox's bundled interface every time. Games load assets again."
            ),
        )
        client_group.add(
            widgets.bind_switch(
                self.window, cache_on_launch, "rosemary_clear_cache_on_launch"
            )
        )
        return client_group

    def _rosemary_source(self):
        return rosemary_install.normalize_source(
            self.window.settings.get("rosemary_source")
        )

    def _show_rosemary_source(self, _key, source):
        with self.rosemary_source_row.handler_block(self._rosemary_source_handler):
            self.rosemary_source_row.set_selected(
                self._rosemary_sources.index(rosemary_install.normalize_source(source))
            )

    def _rosemary_source_selected(self, row, _pspec):
        window = self.window
        kind = self._rosemary_sources[row.get_selected()]
        window.set_setting("rosemary_source", kind)
        # Quadra installs the build you picked when it is not there yet.
        if (
            not rosemary_runtime.quadra_runners(kind)
            and not self._chosen_rosemary_program()
            and not window.busy
            and not window.setup_active
        ):
            self._install_rosemary()

    def _check_rosemary_update(self):
        kind = self._rosemary_source()
        newest = rosemary_install.latest_version(source=kind)
        have = {
            name.removeprefix("Rosemary ")
            for name, _path in rosemary_runtime.quadra_runners(kind)
        }
        # Quadra's build falls back to the regular one while it has no release: not an update.
        if (
            newest
            and have
            and newest not in have
            and rosemary_install.runner_kind(newest) == kind
        ):
            GLib.idle_add(self._offer_rosemary_update, newest)
        return False

    def _offer_rosemary_update(self, version):
        if self.window.settings_page is self:
            self.rosemary_install_button.set_label(
                _("Update to {version}", version=version)
            )
        return False

    def refresh_rosemary(self):
        kind = self._rosemary_source()
        self._fill_rosemary_runner_list()
        try:
            found = rosemary_runtime.find_rosemary(self.window.settings)
            self.rosemary_runner_row.set_subtitle(rosemary_runtime.describe(found))
            has_chosen = bool(rosemary_runtime.quadra_runners(kind))
            custom = bool(self._chosen_rosemary_program())
            self.rosemary_install_button.set_label(
                _("Update") if (has_chosen or custom) else _("Install")
            )
            if has_chosen:
                threading.Thread(
                    target=self._check_rosemary_update, daemon=True
                ).start()
        except rosemary_runtime.RosemaryError as error:
            self.rosemary_runner_row.set_subtitle(str(error).splitlines()[0])
            self.rosemary_install_button.set_label(_("Install"))
        settings = self.window.settings
        try:
            program = rosemary_runtime.find_rosemary(settings)
            self.rosemary_vr_row.set_subtitle(
                _("Ready: turn on VR in the VR tab")
                if rosemary_xr.supports_vr(program)
                else _(
                    'This Rosemary cannot load VR. Choose Quadra\'s Rosemary under "Rosemary build".'
                )
            )
        except rosemary_runtime.RosemaryError:
            self.rosemary_vr_row.set_subtitle(_("Install Rosemary first"))
        self.rosemary_client_row.set_subtitle(
            rosemary_runtime.installed_version(settings) or _("Not installed")
        )
        previous = rosemary_runtime.previous_build(settings)
        pinned = bool(settings.get("rosemary_pinned_build"))
        self.rosemary_rollback_row.set_visible(pinned or previous is not None)
        if pinned:
            self.rosemary_rollback_row.set_subtitle(
                _("Staying on this version; updates are paused")
            )
            self.rosemary_rollback_button.set_label(_("Follow latest"))
        elif previous is not None:
            self.rosemary_rollback_row.set_subtitle(
                previous[0].version or previous[0].guid
            )
            self.rosemary_rollback_button.set_label(_("Go back"))

    def _chosen_rosemary_program(self):
        chosen = (self.window.settings.get("rosemary_program") or "").strip()
        return str(Path(chosen).expanduser()) if chosen else ""

    def _keep_typed_rosemary_program_listed(self):
        typed = self._chosen_rosemary_program()
        settings = self.window.settings
        typed_runner = next(
            (r for r in rosemary_runtime.runner_list(settings) if r.program == typed),
            None,
        )
        added = settings.get("rosemary_runners") or []
        if typed_runner and typed_runner.added_by_hand and typed not in added:
            self.window.set_setting("rosemary_runners", [*added, typed])

    def _fill_rosemary_runner_list(self):
        automatic = rosemary_runtime.Runner("", _("Automatic"), False)
        runners = [automatic, *rosemary_runtime.runner_list(self.window.settings)]
        programs = [runner.program for runner in runners]
        chosen = self._chosen_rosemary_program()
        with self.rosemary_runner_row.handler_block(self._runner_selected_handler):
            if runners != self._rosemary_runners:
                self._rosemary_runners = runners
                self.rosemary_runner_row.set_model(
                    Gtk.StringList.new([runner.label for runner in runners])
                )
            self.rosemary_runner_row.set_selected(
                programs.index(chosen) if chosen in programs else 0
            )
        self._show_remove_runner_button()

    def _selected_rosemary_runner(self):
        return self._rosemary_runners[self.rosemary_runner_row.get_selected()]

    def _show_remove_runner_button(self):
        self.rosemary_remove_runner_button.set_visible(
            self._selected_rosemary_runner().added_by_hand
        )

    def _rosemary_runner_selected(self, _row, _pspec):
        self.window.set_setting(
            "rosemary_program", self._selected_rosemary_runner().program
        )

    def _choose_rosemary_runner(self, on_added=lambda: None):
        def chosen(path):
            if self._add_rosemary_runner(path):
                on_added()

        dialogs._choose_file(
            self.window, _("Choose a Rosemary program"), _("Add"), chosen
        )

    def _add_rosemary_runner(self, program):
        if not rosemary_runtime.is_runnable(program):
            dialogs._error_dialog(
                self.window,
                _("Rosemary"),
                _("{path} is not an executable file.", path=program),
            )
            return False
        window = self.window
        added = [
            p for p in window.settings.get("rosemary_runners") or [] if p != program
        ]
        window.set_setting("rosemary_runners", [*added, program])
        window.set_setting("rosemary_program", program)
        return True

    def _remove_rosemary_runner(self):
        window = self.window
        removed = self._selected_rosemary_runner().program
        kept = [
            p
            for p in window.settings.get("rosemary_runners") or []
            if str(Path(p).expanduser()) != removed
        ]
        window.set_setting("rosemary_runners", kept)
        window.set_setting("rosemary_program", "")

    def _toggle_rosemary_rollback(self):
        settings = self.window.settings
        if settings.get("rosemary_pinned_build"):
            self.window.set_setting("rosemary_pinned_build", "")
        else:
            previous = rosemary_runtime.previous_build(settings)
            if previous is not None:
                self.window.set_setting("rosemary_pinned_build", previous[0].key)

    def _set_rosemary_busy(self, busy):
        for button in (self.rosemary_install_button, self.rosemary_client_button):
            button.set_sensitive(not busy)
        for bar in self.rosemary_progress_bars:
            bar.set_visible(busy)

    def _run_rosemary(self, work):
        """Run `work(progress)` off the GTK thread with the progress bar."""
        if not self.window.begin("installing Rosemary"):
            return
        self._set_rosemary_busy(True)

        def show_progress(fraction, text):
            for bar in self.rosemary_progress_bars:
                bar.set_fraction(max(0.0, min(1.0, fraction)))
                bar.set_text(text)
            return False

        def progress(fraction, text):
            GLib.idle_add(show_progress, fraction, text)

        def run():
            error = None
            try:
                work(progress)
            except Exception as failure:
                error = failure
            GLib.idle_add(self._rosemary_finished, error)

        threading.Thread(target=run, daemon=True).start()

    def _rosemary_finished(self, error):
        self._set_rosemary_busy(False)
        self.window.end(resume_pending=False)
        self.window.client_changed()
        if error is not None:
            dialogs._error_dialog(self.window, _("Rosemary"), str(error) or repr(error))
        else:
            dialogs._toast(self.window.toasts, _("Done"))
        return False

    def _install_rosemary(self):
        source = self._rosemary_source()
        self._run_rosemary(
            lambda progress: rosemary_install.install_latest(progress, source=source)
        )

    def _install_rosemary_client(self):
        settings = self.window.settings.snapshot()
        self._run_rosemary(
            lambda progress: rosemary_runtime.ensure_ready(settings, progress)
        )

    # Keep the stored Metal key so existing selections follow Quadra M2VK.
    RENDERER_CODES = ("opengl", "vulkan", "metal")

    # (code, label, subtitle) of the choices; the first is the default.
    M2VK_ACCURACY = (
        ("fast", "Fast", "The device's own maths and texture decoders."),
        (
            "textures",
            "Textures",
            "Metal's BC texture decode where the driver's may differ. Costs memory.",
        ),
        (
            "exact",
            "Exact",
            "Every exact form: for diagnosing wrong colors or shapes. Slowest.",
        ),
    )
    M2VK_PRESENT = (
        (
            "auto",
            "Automatic",
            "Tear-free with low latency where the driver supports it.",
        ),
        ("mailbox", "Mailbox", "Tear-free; the newest frame replaces a waiting one."),
        ("fifo", "FIFO (vsync)", "Strict vsync. Adds latency; always supported."),
        ("immediate", "Immediate", "Lowest latency; may tear."),
    )

    def _m2vk_combo(self, key, title, subtitle, choices):
        codes = [code for code, _label, _hint in choices]
        row = widgets.ComboRow(
            title=_(title),
            subtitle=_(subtitle),
            model=Gtk.StringList.new([_(label) for _code, label, _hint in choices]),
        )
        widgets.bind_combo(self.window, row, key, codes, codes[0])
        self._m2vk_defaults[key] = codes[0]
        return row

    def _m2vk_switch(self, key, title, subtitle, default):
        row = Adw.SwitchRow(title=title, subtitle=subtitle)
        widgets.bind_switch(self.window, row, key, default)
        self._m2vk_defaults[key] = default
        return row

    def _m2vk_spin(self, key, title, subtitle, low, high, step, default):
        row = Adw.SpinRow.new_with_range(low, high, step)
        row.set_digits(0)
        row.set_title(_(title))
        row.set_subtitle(_(subtitle))

        def in_range(value):
            whole_number = isinstance(value, int) and not isinstance(value, bool)
            return value if whole_number and low <= value <= high else default

        widgets.bind_spin(
            self.window, row, key, default, to_setting=int, to_row=in_range
        )
        self._m2vk_defaults[key] = default
        return row

    def _build_m2vk_group(self, settings):
        """Quadra M2VK's settings: the host library reads them at the next launch."""
        window = self.window
        self._m2vk_defaults = {}
        group = Adw.PreferencesGroup(
            title=_("Quadra M2VK"),
            description=_(
                "Applies on next launch, and only while the renderer is Quadra M2VK. "
                "The defaults suit most computers."
            ),
        )
        self.m2vk_group = group

        shader_preload = Adw.SwitchRow(
            title=_("Shader preloading"),
            subtitle=_(
                "Prepare learned shaders before playing. New shaders may still compile during play."
            ),
        )
        group.add(
            widgets.bind_switch(window, shader_preload, "metal_shader_preload", True)
        )
        for key, title, subtitle in (
            (
                "metal_library_preload",
                "Prepare client shader libraries",
                "Prepare installed shader libraries before playing, including the first launch.",
            ),
            (
                "metal_resource_preload",
                "Preload local textures and fonts",
                "Warm bundled resources before playing. Downloaded world assets still load in game.",
            ),
        ):
            row = Adw.SwitchRow(title=_(title), subtitle=_(subtitle))
            group.add(widgets.bind_switch(window, row, key, True))
            row.set_sensitive(shader_preload.get_active())
            shader_preload.connect(
                "notify::active",
                lambda switch, _prop, row=row: row.set_sensitive(switch.get_active()),
            )

        group.add(
            self._m2vk_combo(
                "metal_accuracy",
                "Accuracy",
                "Fast uses the device's own maths and texture decoders. Textures decodes BC textures as Metal does: try it "
                "if textures look wrong. Exact is slowest, for diagnosing wrong colors or shapes.",
                self.M2VK_ACCURACY,
            )
        )
        group.add(
            self._m2vk_combo(
                "metal_present_mode",
                "Frame presentation",
                "How finished frames reach the screen. Try FIFO if the picture stutters or tears.",
                self.M2VK_PRESENT,
            )
        )
        group.add(
            self._m2vk_switch(
                "metal_memory_priority",
                _("Keep important graphics memory resident"),
                _(
                    "When video memory runs out, let the driver move textures aside before render targets "
                    "instead of failing. Turn off if a driver misbehaves."
                ),
                True,
            )
        )
        group.add(
            self._m2vk_spin(
                "metal_vram_limit_gib",
                "Graphics memory limit (GiB)",
                "0 uses what the card has free at launch. A limit only lowers it; try 4 on a busy 6 GiB card.",
                0,
                256,
                1,
                0,
            )
        )
        group.add(
            self._m2vk_spin(
                "metal_compile_threads",
                "Shader compile threads",
                "0 chooses from your processor and free memory. Fewer lowers memory use during compiles.",
                0,
                32,
                1,
                0,
            )
        )
        group.add(
            self._m2vk_spin(
                "metal_warm_states",
                "Learned shaders kept ready",
                "Up to this many, within a quarter of free memory. More smooths areas you have visited before.",
                0,
                1024,
                32,
                256,
            )
        )

        group.add(
            self._m2vk_switch(
                "metal_fallback",
                _("Fall back to OpenGL if Quadra M2VK cannot start"),
                _(
                    "Play with OpenGL this time instead of stopping, when no working GPU driver is found."
                ),
                True,
            )
        )

        diagnostics = Adw.SwitchRow(
            title=_("Quadra M2VK performance diagnostics"),
            subtitle=_(
                "Record compilation, encoding, GPU timing and memory use for performance reports."
            ),
        )
        group.add(widgets.bind_switch(window, diagnostics, "metal_diagnostics"))

        reset = widgets._button_row(_("Reset Quadra M2VK settings"))
        reset.connect("activated", lambda *_args: self._reset_m2vk())
        group.add(reset)
        self._update_m2vk(settings.get("renderer", "opengl"))

    def _reset_m2vk(self):
        for key, default in self._m2vk_defaults.items():
            self.window.set_setting(key, default)
        dialogs._toast(self.window.toasts, _("Quadra M2VK settings reset"))

    def _update_m2vk(self, renderer):
        """The group applies only while Quadra M2VK is the renderer."""
        group = getattr(self, "m2vk_group", None)
        if group is not None:
            group.set_sensitive(renderer == "metal")

    def _renderer_row(self):
        row = widgets.ComboRow(
            title=_("Renderer"),
            subtitle=_(
                "Applies on next launch. Zink uses Roblox's OpenGL renderer; Quadra M2VK uses its Metal renderer on Vulkan. VR uses OpenGL or Zink."
            ),
            model=Gtk.StringList.new(
                [
                    _("OpenGL"),
                    _("Vulkan (Zink, experimental)"),
                    _("Quadra M2VK (experimental)"),
                ]
            ),
        )
        selected = self.window.settings.get("renderer", "opengl")
        row.set_selected(
            self.RENDERER_CODES.index(selected)
            if selected in self.RENDERER_CODES
            else 0
        )
        self._renderer_rows.append(
            (row, row.connect("notify::selected", self._select_renderer))
        )
        if len(self._renderer_rows) == 1:
            self.window.watch_setting(
                "renderer", lambda _key, code: self._show_renderer(code)
            )
        return row

    def _show_renderer(self, code):
        self._update_m2vk(code)
        index = self.RENDERER_CODES.index(code) if code in self.RENDERER_CODES else 0
        for row, handler in self._renderer_rows:
            if row.get_selected() != index:
                row.handler_block(handler)
                row.set_selected(index)
                row.handler_unblock(handler)

    def _select_renderer(self, row, _pspec):
        window = self.window
        selected = self.RENDERER_CODES[row.get_selected()]
        from .. import graphics

        if selected == "metal":
            if core.PREBUILT_SHIM and not core.metal_built():
                self._show_renderer(window.settings.get("renderer", "opengl"))
                dialogs._error_dialog(
                    window,
                    _("Quadra M2VK is unavailable"),
                    _("Quadra M2VK is not part of this package"),
                )
                return
            missing = graphics.missing_metal_dependencies()
            if missing:
                self._show_renderer(window.settings.get("renderer", "opengl"))
                dialogs._error_dialog(
                    window,
                    _("Quadra M2VK is unavailable"),
                    _("Install these first: {programs}", programs=", ".join(missing)),
                )
                return
            window.set_setting("renderer", selected)
            return
        if selected == "opengl" or not graphics.missing_vulkan_dependencies():
            window.set_setting("renderer", selected)
            return
        for other, _handler in self._renderer_rows:
            other.set_sensitive(False)
        dialogs._toast(
            window.toasts,
            _(
                "Installing Vulkan dependencies… Authorize the administrator prompt to continue."
            ),
        )

        def installed(_result, error):
            for other, _handler in self._renderer_rows:
                other.set_sensitive(True)
            if error:
                self._show_renderer(window.settings.get("renderer", "opengl"))
                dialogs._error_dialog(
                    window, _("Could not install Vulkan dependencies"), str(error)
                )
            else:
                window.set_setting("renderer", "vulkan")
                self._show_renderer("vulkan")
                dialogs._toast(window.toasts, _("Vulkan dependencies installed"))

        self._in_thread(graphics.ensure_vulkan_dependencies, installed)

    def _gpu_row(self, always_visible=False):
        row = widgets.ComboRow(
            title=_("Graphics card"),
            subtitle=_("Detecting graphics cards… Applies on next launch."),
            model=Gtk.StringList.new([_("Automatic")]),
            sensitive=False,
        )
        self._gpu_rows.append(
            (row, row.connect("notify::selected", self._select_gpu), always_visible)
        )
        if len(self._gpu_rows) == 1:
            from .. import gpus

            self.window.watch_setting("gpu", lambda _key, code: self._show_gpu(code))
            self._gpu_choice_needed = self.window.settings.get("gpu", "auto") != "auto"
            self._in_thread(gpus.enumerate_gpus, self._gpu_choices_loaded)
        self._show_gpu_rows()
        return row

    def _show_gpu_rows(self):
        renderer_applies = backends.supports(self.window.runtime_name(), "renderer")
        for row, _handler, always_visible in self._gpu_rows:
            row.set_visible(
                always_visible or (renderer_applies and self._gpu_choice_needed)
            )

    def _select_gpu(self, row, _pspec):
        selected = row.get_selected()
        if selected < len(self._gpu_codes):
            self.window.set_setting("gpu", self._gpu_codes[selected])

    def _show_gpu(self, code):
        if code not in self._gpu_codes:
            return
        index = self._gpu_codes.index(code)
        for row, handler, _always in self._gpu_rows:
            if row.get_selected() != index:
                with row.handler_block(handler):
                    row.set_selected(index)

    def _gpu_choices_loaded(self, adapters, error):
        if getattr(self.window, "settings_page", None) is not self:
            return False
        selection = self.window.settings.get("gpu", "auto")
        adapters = adapters or []
        codes = ["auto", *(adapter["id"] for adapter in adapters)]
        labels = [_("Automatic"), *(adapter["label"] for adapter in adapters)]
        if selection not in codes:
            codes.append(selection)
            labels.append(_("Selected graphics card unavailable"))
        self._gpu_codes = codes
        self._gpu_choice_needed = (
            len(adapters) > 1 or selection != "auto" or error is not None
        )
        self._show_gpu_rows()
        for row, handler, _always_visible in self._gpu_rows:
            row.handler_block(handler)
            try:
                row.set_model(Gtk.StringList.new(labels))
                row.set_selected(codes.index(selection))
            finally:
                row.handler_unblock(handler)
            row.set_sensitive(True)
            row.set_subtitle(
                _(
                    "Could not detect graphics cards. Select Automatic to use the system default."
                )
                if error
                else _(
                    "Applies on next launch. Automatic keeps your desktop or terminal's GPU selection."
                )
            )
        return False

    def _allow_debugger_row(self):
        return self._switch_row(
            "allow_debugger",
            _("Allow freeze diagnostics"),
            _(
                "QUADRA_ALLOW_DEBUGGER. Lets Collect diagnostics read the game's thread stacks; "
                "keeps the crash handler available. Turn on before starting the game."
            ),
        )

    def _text_input_trace_row(self):
        return self._switch_row(
            "trace_text_input",
            _("Trace text input"),
            _(
                "Records insertion counts and selection ranges to diagnose duplicate typing. No typed text is logged. Applies on next launch."
            ),
        )

    def _switch_row(self, key, title, subtitle):
        row = Adw.SwitchRow(title=title, subtitle=subtitle)
        return widgets.bind_switch(self.window, row, key)

    def _ciderdarling_page(self):
        window = self.window
        page = Adw.PreferencesPage()
        about = Adw.PreferencesGroup(
            title="CiderDarling",
            description=_(
                "These settings apply while CiderDarling is the active runtime. The renderer "
                "and graphics card are shared with Environment → Game."
            ),
        )
        self.cider_revision_row = Adw.ActionRow(title=_("Installed revision"))
        self.cider_revision_row.set_subtitle_selectable(True)
        self.cider_update_button = Gtk.Button(
            label=_("Update in terminal"), valign=Gtk.Align.CENTER, visible=False
        )
        self.cider_update_button.add_css_class("suggested-action")
        self.cider_update_button.connect(
            "clicked", lambda *_args: window.install_ciderdarling()
        )
        self.cider_revision_row.add_suffix(self.cider_update_button)
        self.cider_check_button = Gtk.Button(
            icon_name="view-refresh-symbolic",
            valign=Gtk.Align.CENTER,
            tooltip_text=_("Check for CiderDarling updates"),
            visible=not runtime.is_bundled(),
        )
        self.cider_check_button.connect(
            "clicked", lambda *_args: window.check_ciderdarling_updates(force=True)
        )
        self.cider_revision_row.add_suffix(self.cider_check_button)
        about.add(self.cider_revision_row)
        self.cider_active_row = Adw.ActionRow(title=_("Active runtime"))
        about.add(self.cider_active_row)
        automatic = Adw.SwitchRow(
            title=_("Check for CiderDarling updates automatically"),
            subtitle=_(
                "Checks in the background. Install an available update in a separate terminal."
            ),
            sensitive=not runtime.is_bundled(),
        )
        about.add(
            widgets.bind_switch(
                window, automatic, "auto_check_ciderdarling_updates", True
            )
        )
        self.cider_rollback_row = widgets._button_row(
            _("Use previous CiderDarling and restart")
        )
        self.cider_rollback_row.set_tooltip_text(
            _(
                "Keeps this prefix and sign-in. The previous installation must still exist."
            )
        )
        self.cider_rollback_row.connect(
            "activated", lambda *_args: self.rollback_runtime()
        )
        about.add(self.cider_rollback_row)
        page.add(about)

        graphics = Adw.PreferencesGroup(title=_("Graphics"))
        graphics.add(self._renderer_row())
        graphics.add(self._gpu_row(always_visible=True))
        page.add(graphics)

        threads = Adw.PreferencesGroup(title=_("Threads"))
        kicks = Adw.SwitchRow(
            title=_("Watchdog thread kicks"),
            subtitle=_(
                "QUADRA_KICK. Interrupts threads stuck in long waits. Off by default: "
                "each kick costs time. Applies on next launch."
            ),
        )
        threads.add(widgets.bind_switch(window, kicks, "cider_thread_kicks"))
        page.add(threads)

        diagnostics = Adw.PreferencesGroup(title=_("Diagnostics"))
        trace = Adw.SwitchRow(
            title=_("Record runtime trace"),
            subtitle=_(
                "CIDER_TRACE_FILE. Numeric timings, no game text, in a private file in the logs "
                "folder (up to 32 MB per launch). Applies on next launch."
            ),
        )
        diagnostics.add(widgets.bind_switch(window, trace, "cider_trace"))
        verbose = Adw.SwitchRow(
            title=_("Verbose darlingserver log"),
            subtitle=_(
                "DSERVER_LOG_LEVEL=debug. dserver.log grows quickly; applies the next time "
                "Darling starts."
            ),
        )
        diagnostics.add(
            widgets.bind_switch(window, verbose, "cider_verbose_server_log")
        )
        diagnostics.add(self._allow_debugger_row())
        diagnostics.add(self._text_input_trace_row())
        collect = widgets._button_row(_("Collect diagnostics"))
        collect.connect("activated", lambda *_args: window.collect_diagnostics())
        diagnostics.add(collect)
        hardware = widgets._button_row(_("Collect hardware report"))
        hardware.set_tooltip_text(
            _(
                "Tests GPU rendering, MSAA, CPU limits and window resources. Saves a local report."
            )
        )
        hardware.connect("activated", lambda *_args: window.collect_hardware_report())
        diagnostics.add(hardware)
        prefix = widgets._button_row(_("Open prefix folder"))
        prefix.connect("activated", lambda *_args: self.open_prefix())
        diagnostics.add(prefix)
        restart = widgets._button_row(_("Restart Darling"))
        restart.connect("activated", lambda *_args: self.restart_darling())
        diagnostics.add(restart)
        page.add(diagnostics)
        return page

    def refresh_ciderdarling(self):
        """Show the CiderDarling tab and its shortcut while it is installed."""
        available = "ciderdarling" in self._runtime_choices and not os.environ.get(
            "FLATPAK_ID"
        )
        self.cider_stack_page.set_visible(available)
        self.cider_settings_row.set_visible(available)
        if not available:
            if self.stack.get_visible_child_name() == "ciderdarling":
                self.stack.set_visible_child_name("env")
            return
        installed, pinned = self.ciderdarling_revisions()
        update = self.ciderdarling_update_needed()
        if update:
            subtitle = _(
                "{installed}; update {pinned} available",
                installed=installed[:12],
                pinned=pinned[:12],
            )
        else:
            subtitle = (installed or _("unknown"))[:12]
        self.cider_revision_row.set_subtitle(subtitle)
        self.cider_update_button.set_visible(update)
        self.cider_update_button.set_sensitive(self.window.runtime_install_job is None)
        self.cider_check_button.set_sensitive(not self.window.cider_update_checking)
        try:
            previous = (
                None
                if runtime.is_bundled()
                else runtime.rollback_candidate(core.DARLING_RUNTIME)
            )
        except (OSError, RuntimeError, ValueError):
            previous = None
        self.cider_rollback_row.set_visible(previous is not None)
        self.cider_rollback_row.set_sensitive(self.window.runtime_install_job is None)
        active = core.DARLING_RUNTIME["runtime"] == "ciderdarling"
        self.cider_active_row.set_subtitle(
            self._runtime_labels["ciderdarling"]
            if active
            else _(
                "{runtime}. Switch in Environment → Darling to use these settings.",
                runtime=self._runtime_labels[core.DARLING_RUNTIME["runtime"]],
            )
        )

    def open_prefix(self):
        try:
            Gio.AppInfo.launch_default_for_uri(core.DARLING_PREFIX.as_uri(), None)
        except (OSError, GLib.Error, ValueError) as error:
            dialogs._toast(
                self.window.toasts,
                _("Could not open the prefix folder: {error}", error=error),
            )

    def open_logs(self):
        try:
            core.LOGS.mkdir(parents=True, exist_ok=True)  # none before the first game
            Gio.AppInfo.launch_default_for_uri(core.LOGS.as_uri(), None)
        except (OSError, GLib.Error) as error:
            dialogs._toast(
                self.window.toasts,
                _("Could not open the logs folder: {error}", error=error),
            )

    def _in_thread(self, work, done):
        def run():
            try:
                result = work()
                GLib.idle_add(done, result, None)
            except Exception as error:  # shown to the user
                GLib.idle_add(done, None, error)

        threading.Thread(target=run, daemon=True).start()

    def _set_update_action(self, label, action):
        self.update_button.set_label(label)
        self.update_button.disconnect(self._update_handler)
        self._update_handler = self.update_button.connect(
            "clicked", lambda *_args: action()
        )

    def check_launcher_update(self):
        self.check_launcher_btn.set_sensitive(False)
        self.check_launcher_btn.set_label(_("Checking…"))

        def done(result, error):
            self.check_launcher_btn.set_sensitive(True)
            if error:
                self.check_launcher_btn.set_label(_("Check for updates"))
                dialogs._toast(
                    self.window.toasts, _("Could not check: {error}", error=error)
                )
                return
            has_update, tag, html_url = result
            if has_update:
                self.launcher_version_row.set_subtitle(
                    _("Update {version} available", version=tag)
                )
                self.check_launcher_btn.set_label(_("Update to {version}", version=tag))
                self.check_launcher_btn.disconnect(self._launcher_update_handler)
                self._launcher_update_handler = self.check_launcher_btn.connect(
                    "clicked", lambda *_args: self.force_update_launcher()
                )
                dialogs._toast(
                    self.window.toasts, _("Update {version} available", version=tag)
                )
            else:
                self.check_launcher_btn.set_label(_("Check for updates"))
                dialogs._toast(self.window.toasts, _("Quadra is up to date"))

        self._in_thread(core.check_launcher_update, done)

    def force_update_launcher(self):
        if not self.window.begin("updating_launcher"):
            return
        self.check_launcher_btn.set_sensitive(False)
        self.launcher_progress.set_visible(True)
        self.launcher_progress.set_fraction(0.1)
        self.launcher_progress.set_text(_("Updating…"))

        def progress(fraction, text):
            GLib.idle_add(self.launcher_progress.set_fraction, fraction)
            GLib.idle_add(self.launcher_progress.set_text, text)

        def done(result, error):
            self.window.end()
            self.check_launcher_btn.set_sensitive(True)
            self.launcher_progress.set_visible(False)
            if error:
                dialogs._error_dialog(
                    self.window, _("Update failed"), str(error) or repr(error)
                )
                return
            ok, msg = result if result else (False, "")
            if ok:
                dialogs._toast(self.window.toasts, msg)
            else:
                system._open_uri(self.window, msg)

        self._in_thread(lambda: core.update_launcher(progress), done)

    def check_updates(
        self,
        install=False,
        on_progress=None,
        on_complete=None,
        force_download=False,
        channel="",
    ):
        """Looks for a newer client; with install=True also installs it
        using the same asynchronous workflow as first-launch setup."""
        if self._checking_updates:
            return False
        if install and not self.window.begin("updating"):
            return False
        self._checking_updates = True
        self.update_button.set_sensitive(False)
        self.update_button.set_label(_("Checking…"))
        # Setup and ordinary checks always use the public release. Only the
        # required-update dialog can explicitly request a rollout channel.

        def done(result, error):
            self._checking_updates = False
            self.update_button.set_sensitive(True)
            if error:
                self.update_button.set_label(_("Check for updates"))
                if install:
                    self.window.end(resume_pending=False)
                if on_complete:
                    on_complete(error)
                else:
                    dialogs._toast(
                        self.window.toasts, _("Could not check: {error}", error=error)
                    )
                return
            version, upload = result
            installed = core.installed_version()
            if (
                installed
                and not core.version_is_newer(version, installed)
                and not force_download
            ):
                self.update_button.set_label(_("Check for updates"))
                if install:
                    # A previous attempt may have downloaded Roblox but failed
                    # during preparation. Retry preparation without downloading
                    # or replacing that already-current bundle again.
                    self.install_update(
                        upload,
                        on_progress=on_progress,
                        on_complete=on_complete,
                        _claimed=True,
                        _prepare_only=True,
                    )
                else:
                    dialogs._toast(
                        self.window.toasts,
                        _("No newer version was reported for this channel"),
                    )
                return
            self._set_update_action(
                _("Update to {version}", version=version),
                lambda: self.install_update(
                    upload, expected_version=version, channel=channel
                ),
            )
            if install:
                self.install_update(
                    upload,
                    on_progress=on_progress,
                    on_complete=on_complete,
                    _claimed=True,
                    expected_version=version,
                    channel=channel,
                )

        self._in_thread(
            lambda: core.latest_version(channel, required_log=self.window.last_log),
            done,
        )
        return True

    def install_update(
        self,
        upload,
        on_progress=None,
        on_complete=None,
        _claimed=False,
        _prepare_only=False,
        expected_version=None,
        channel=None,
    ):
        if not _claimed and not self.window.begin("updating"):
            return False
        self.update_button.set_sensitive(False)
        self.progress.set_visible(True)
        shown = [-1.0, ""]

        def progress(fraction, text):
            # One update per half percent, not two per 64 KiB chunk.
            if (
                fraction - shown[0] < 0.005
                and text[:12] == shown[1][:12]
                and fraction < 1
            ):
                return
            shown[:] = [fraction, text]

            def show_progress():
                self.progress.set_fraction(fraction)
                self.progress.set_text(text)
                if on_progress:
                    on_progress(fraction, text)

            GLib.idle_add(show_progress)

        def done(backup, error):
            if error is None and not _prepare_only and channel is not None:
                self.window.set_setting("roblox_channel", channel)
            self.window.end(resume_pending=error is None)
            self.update_button.set_sensitive(True)
            self._set_update_action(_("Check for updates"), self.check_updates)
            self.progress.set_visible(False)
            self.window.client_changed()
            if error is None:
                core.clear_last_error()
            if on_complete:
                on_complete(error)
            elif error:
                dialogs._error_dialog(
                    self.window, _("Update failed"), str(error) or repr(error)
                )
            else:
                dialogs._toast(
                    self.window.toasts,
                    _("Roblox updated, the old version is in backups/")
                    if backup
                    else _("Roblox installed"),
                )

        # A fresh client does not start until the shim is rebuilt and the
        # Darling prefix is restarted (stale launchd and daemons from the old
        # client), so both run as part of the same progress bar.
        running = self.window.session is not None

        def work():
            backup = None
            if not _prepare_only:
                backup = core.update_roblox(
                    upload,
                    lambda fraction, text: progress(fraction * 0.93, text),
                    expected_version=expected_version,
                )
            progress(0.94, _("Rebuilding the shim…"))
            ok, output = core.build_shim()
            if not ok:
                raise RuntimeError(output)
            if not running:
                progress(0.97, _("Restarting Darling…"))
                core.restart_darling()
            progress(1.0, _("Done"))
            return backup

        self._in_thread(work, done)
        return True

    def clear_roblox_cache(self):
        prefix = rosemary_runtime.prefix_path(self.window.settings)
        if not self.window.begin("clearing the cache"):
            return

        def done(freed, error):
            self.window.end()
            if error:
                dialogs._error_dialog(
                    self.window, _("Could not clear the cache"), str(error)
                )
                return
            dialogs._toast(
                self.window.toasts,
                _("Roblox cache cleared ({size} MB)", size=round(freed / 1_000_000)),
            )

        self._in_thread(lambda: rosemary_runtime.clear_roblox_cache(prefix), done)

    def logout(self):
        dialog = Adw.AlertDialog(
            heading=_("Sign out?"),
            body=_(
                "The saved Roblox session will be deleted, you will need to sign in again next time."
            ),
        )
        dialog.add_response("cancel", _("Cancel"))
        dialog.add_response("logout", _("Sign out of Roblox"))
        dialog.set_response_appearance("logout", Adw.ResponseAppearance.DESTRUCTIVE)

        def response(_dialog, result):
            # A running game would write its session back on the next cookie change.
            if result != "logout" or not self.window.begin("signing out"):
                return

            def done(gone, error):
                self.window.end()
                if gone:
                    dialogs._toast(self.window.toasts, _("Session deleted"))
                else:
                    dialogs._error_dialog(
                        self.window,
                        _("Could not sign out"),
                        str(error)
                        if error
                        else _(
                            "The saved session is still there. Press Restart Darling in "
                            "Diagnostics and try again."
                        ),
                    )

            if self.window.rosemary_selected():
                prefix = rosemary_runtime.prefix_path(self.window.settings)
                self._in_thread(lambda: rosemary_runtime.sign_out(prefix) or True, done)
            else:
                self._in_thread(core.logout, done)

        dialog.connect("response", response)
        dialog.present(self.window)

    def _throttle_state_done(self, state, _error):
        if not hasattr(self, "throttle_row"):
            return
        row = self.throttle_row

        # Adw.ButtonRow has no subtitle on libadwaita < 1.7; the state goes
        # into the button text there and into the subtitle where supported.
        def say(title, subtitle):
            row.set_title(title)
            if hasattr(row, "set_subtitle"):
                row.set_subtitle(subtitle)

        row.set_sensitive(state != "unsupported")
        if state == "patched":
            say(
                _("Remove the throttle patch"),
                _("Applied: the menu renders at full speed from the first second"),
            )
            row.add_css_class("destructive-action")
        elif state == "original":
            say(
                _("Install the throttle patch"),
                _("Not applied: the menu may run at ~3 FPS for the first 10 seconds"),
            )
            row.remove_css_class("destructive-action")
        else:
            say(_("Not available for this Roblox build"), "")

    def toggle_throttle_patch(self):
        if not self.window.begin("patching"):
            return
        remove = core.throttle_patch_state() == "patched"

        def work():
            # The button is the manual path: run the patcher directly, not
            # apply_throttle_patch, which is the automatic path and does
            # nothing while auto_patch_throttle is off.
            return (
                core.remove_throttle_patch() if remove else core.run_throttle_patcher()
            )

        def done(_result, error):
            self.window.end()
            if error:
                dialogs._error_dialog(
                    self.window, _("Throttle patch failed"), str(error) or repr(error)
                )
            else:
                # Keep the switch in step with the button: a hand-removed
                # patch must not be re-applied on the next launch, and a
                # hand-installed one is worth keeping that way.
                self.window.set_setting("auto_patch_throttle", not remove)
            self._in_thread(core.throttle_patch_state, self._throttle_state_done)

        self._in_thread(work, done)

    def delete_roblox(self):
        if self.window.session:
            dialogs._toast(self.window.toasts, _("Close Roblox first"))
            return
        if not core.installed_version():
            dialogs._toast(self.window.toasts, _("Roblox is not installed"))
            return
        dialog = Adw.AlertDialog(
            heading=_("Delete Roblox?"),
            body=_(
                "RobloxPlayer.app and all mod modifications will be removed from your computer."
            ),
        )
        dialog.add_response("cancel", _("Cancel"))
        dialog.add_response("delete", _("Delete"))
        dialog.set_response_appearance("delete", Adw.ResponseAppearance.DESTRUCTIVE)

        def response(_dialog, result):
            if result != "delete" or not self.window.begin("deleting Roblox"):
                return
            try:
                core.delete_roblox()
            finally:
                self.window.end(resume_pending=False)
                self.window.client_changed()
            dialogs._toast(self.window.toasts, _("Roblox deleted successfully"))

        dialog.connect("response", response)
        dialog.present(self.window)

    def rebuild(self):
        if not self.window.begin("building"):
            return
        dialogs._toast(self.window.toasts, _("Building the shim…"))

        def done(result, error):
            self.window.end()
            ok, output = result if result else (False, str(error))
            if ok:
                dialogs._toast(self.window.toasts, _("Shim built"))
            else:
                dialogs._error_dialog(
                    self.window, _("Could not build the shim"), output
                )

        self._in_thread(core.build_shim, done)

    def restart_darling(self):
        if not self.window.begin("restarting"):
            return

        def done(_result, error):
            self.window.end()
            if error:
                dialogs._toast(
                    self.window.toasts,
                    _("Could not restart Darling: {error}", error=error),
                )
            else:
                dialogs._toast(
                    self.window.toasts,
                    _("Darling stopped, it starts with the next game"),
                )

        self._in_thread(core.restart_darling, done)
