"""First-launch setup pages: the theme, the runtime that plays Roblox, and
installing it, sharing the launcher's Roblox installer."""

import threading

import gi

gi.require_version("Gtk", "4.0")
gi.require_version("Adw", "1")
from gi.repository import Adw, Gdk, GLib, Gtk, Pango

from . import __version__, backends, core, theme
from .darling import runtime as darling_runtime
from .darling import runtime_install
from .i18n import _
from .paths import user_directory
from .rosemary import install as rosemary_install
from .rosemary import runtime as rosemary_runtime

_STYLE = """
.setup-wizard {
  background-image: linear-gradient(155deg, alpha(@accent_bg_color, 0.10), transparent 65%);
}
.setup-wizard .setup-title { font-size: 30px; font-weight: 800; letter-spacing: -0.6px; }
.setup-wizard .setup-subtitle { font-size: 15px; }
.setup-wizard .setup-eyebrow { font-size: 11px; font-weight: 700; letter-spacing: 1.2px; }
.setup-wizard .setup-step { padding: 16px; border-radius: 14px; }
.setup-wizard .setup-number {
  min-width: 30px; min-height: 30px; border-radius: 50%;
  background-color: alpha(@accent_bg_color, 0.15); color: @accent_color; font-weight: 700;
}
.setup-wizard .setup-note { padding: 14px 16px; border-radius: 12px; }
.setup-wizard .setup-action { min-height: 38px; padding-left: 24px; padding-right: 24px; }
.setup-wizard .setup-path { font-family: monospace; font-size: 11px; }
.setup-wizard .setup-progress { min-height: 8px; }
"""
_provider = None


def _install_style():
    global _provider
    if _provider is None:
        _provider = Gtk.CssProvider()
        theme.follow_color_scheme(_provider)
        _provider.load_from_string(_STYLE)
        Gtk.StyleContext.add_provider_for_display(
            Gdk.Display.get_default(),
            _provider,
            Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION,
        )


def _label(text, *, classes=(), centered=False):
    return Gtk.Label(
        label=_(text),
        wrap=True,
        xalign=0.5 if centered else 0,
        justify=Gtk.Justification.CENTER if centered else Gtk.Justification.LEFT,
        css_classes=list(classes),
    )


APPEARANCES = (
    ("grid", "Grid", "A tiled backdrop behind the launcher."),
    ("gnome", "GNOME (classic)", "Follows your system's light or dark appearance."),
    ("monochrome", "Monochrome · aubree.lat", "Black and white, with a soft backdrop."),
)
RUNTIMES = (
    (
        "rosemary",
        "Rosemary",
        "Recommended. Quadra downloads it for you: nothing to build.",
    ),
    (
        "darling",
        "Darling (outdated)",
        "Installs Darling and its tools in a terminal, which may ask "
        "for your password. Only Darling has some settings, such as the renderer and DNS.",
    ),
)
DARLING_VARIANTS = (
    ("regular", "Regular Darling", "Your distribution's or DarlingHQ's packages."),
    (
        "ciderdarling",
        "QuadraHQ's CiderDarling",
        "Includes QuadraHQ's threading and graphics changes. "
        "Can need a long source build, and its own first sign-in.",
    ),
)
PREVIOUS_PAGE = {
    "appearance": "welcome",
    "runtime": "appearance",
    "overview": "runtime",
}


class SetupWizard(Adw.Bin):
    """Welcome, theme, runtime, installation overview, progress and ready screens."""

    def __init__(self, window):
        super().__init__()
        self.window = window
        self.installing = False
        self.install_failed = False
        self.darling_job = None
        self.darling_variant = (
            window.settings.get("setup_darling_variant")
            or core.DARLING_RUNTIME["runtime"]
        )
        self.rosemary_bundled = bool(backends.bundled_rosemary())
        _install_style()
        self.add_css_class("setup-wizard")
        toolbar = Adw.ToolbarView()
        header = Adw.HeaderBar()
        header.set_title_widget(Gtk.Label(label="Quadra", css_classes=["heading"]))
        self.back = Gtk.Button(
            icon_name="go-previous-symbolic", tooltip_text=_("Back"), visible=False
        )
        self.back.add_css_class("flat")
        self.back.connect(
            "clicked",
            lambda *_args: self.show(
                self._previous_page(self.pages.get_visible_child_name())
            ),
        )
        header.pack_start(self.back)
        header.pack_end(
            Gtk.Label(label=f"v{__version__}", css_classes=["caption", "dim-label"])
        )
        toolbar.add_top_bar(header)

        self.pages = Gtk.Stack(
            transition_type=Gtk.StackTransitionType.SLIDE_LEFT_RIGHT,
            transition_duration=180,
            vexpand=True,
        )
        self.pages.add_named(self._welcome(), "welcome")
        self.pages.add_named(self._appearance(), "appearance")
        self.pages.add_named(self._runtime(), "runtime")
        self.pages.add_named(self._overview(), "overview")
        self.pages.add_named(self._progress_page(), "progress")
        self.pages.add_named(self._ready(), "ready")
        toolbar.set_content(self.pages)
        self.set_child(toolbar)
        self.show("welcome")

    def _page(self, title, description, icon=None):
        body = Gtk.Box(
            orientation=Gtk.Orientation.VERTICAL,
            spacing=18,
            margin_top=28,
            margin_bottom=28,
            margin_start=24,
            margin_end=24,
            valign=Gtk.Align.CENTER,
            vexpand=True,
        )
        if icon:
            image = Gtk.Image(icon_name=icon, pixel_size=56, halign=Gtk.Align.CENTER)
            image.add_css_class("accent")
            body.append(image)
        body.append(_label(title, classes=("setup-title",), centered=True))
        body.append(
            _label(description, classes=("setup-subtitle", "dim-label"), centered=True)
        )
        clamp = Adw.Clamp(maximum_size=580, tightening_threshold=420, child=body)
        scroll = Gtk.ScrolledWindow(hscrollbar_policy=Gtk.PolicyType.NEVER, child=clamp)
        return scroll, body

    def _actions(self, primary, callback, secondary=None, secondary_callback=None):
        actions = Gtk.Box(spacing=10, halign=Gtk.Align.CENTER, margin_top=6)
        if secondary:
            button = Gtk.Button(
                label=_(secondary), css_classes=["flat", "setup-action"]
            )
            button.connect("clicked", secondary_callback)
            actions.append(button)
        button = Gtk.Button(
            label=_(primary), css_classes=["suggested-action", "pill", "setup-action"]
        )
        button.connect("clicked", callback)
        actions.append(button)
        return actions, button

    def _welcome(self):
        page, body = self._page(
            "Welcome to Quadra", "Play Roblox on your Linux desktop."
        )
        logo = theme.Logo(core.ICONS / "quadrahq-128.png", 88)
        logo.set_halign(Gtk.Align.CENTER)
        body.prepend(logo)
        body.append(
            _label(
                "A short setup picks a look and the runtime that plays Roblox, then installs "
                "them. You’ll sign in inside Roblox when you’re done.",
                centered=True,
            )
        )
        actions, _button = self._actions(
            "Get started",
            lambda *_args: self.show("appearance"),
            "Not now",
            lambda *_args: self.window.dismiss_setup(),
        )
        body.append(actions)
        body.append(
            _label(
                "An independent launcher for Roblox on Linux",
                classes=("caption", "dim-label"),
                centered=True,
            )
        )
        return page

    @staticmethod
    def _choices(options, selected, on_selected):
        group = Adw.PreferencesGroup()
        first = None
        for code, title, description in options:
            check = Gtk.CheckButton(active=code == selected, valign=Gtk.Align.CENTER)
            if first is None:
                first = check
            else:
                check.set_group(first)
            check.connect(
                "toggled",
                lambda button, chosen=code: button.get_active() and on_selected(chosen),
            )
            row = Adw.ActionRow(
                title=_(title), subtitle=_(description), activatable_widget=check
            )
            row.add_prefix(check)
            group.add(row)
        return group

    def _appearance(self):
        page, body = self._page(
            "Choose a look",
            "It applies right away. You can change it later in Settings.",
            "applications-graphics-symbolic",
        )
        current = self.window.settings.get("launcher_appearance", "gnome")
        body.append(self._choices(APPEARANCES, current, self._appearance_selected))
        actions, _button = self._actions(
            "Continue", lambda *_args: self.show(self._page_after_appearance())
        )
        body.append(actions)
        return page

    def _appearance_selected(self, code):
        self.window.set_launcher_appearance(code)

    def _runtime(self):
        page, body = self._page(
            "Choose how Roblox runs",
            "The runtime runs the macOS Roblox client on Linux. "
            "You can switch later in Settings → Environment.",
            "applications-system-symbolic",
        )
        current = "rosemary" if self.window.rosemary_selected() else "darling"
        body.append(self._choices(RUNTIMES, current, self._runtime_selected))
        self.rosemary_sources = self._rosemary_source_group()
        self.rosemary_sources.set_visible(current == "rosemary")
        body.append(self.rosemary_sources)
        self.darling_variants = self._choices(
            DARLING_VARIANTS, self.darling_variant, self._darling_variant_selected
        )
        self.darling_variants.set_title(_("Which Darling"))
        self.darling_variants.set_visible(
            current == "darling" and not darling_runtime.is_bundled()
        )
        body.append(self.darling_variants)
        actions, self.runtime_continue = self._actions(
            "Continue", lambda *_args: self.show("overview")
        )
        body.append(actions)
        self._show_rosemary_source()
        return page

    def _runtime_selected(self, code):
        self.window.set_setting("runtime_backend", code)
        self.rosemary_sources.set_visible(code == "rosemary")
        self.darling_variants.set_visible(
            code == "darling" and not darling_runtime.is_bundled()
        )
        self._show_rosemary_source()

    def _rosemary_source_group(self):
        group = Adw.PreferencesGroup(title=_("Which Rosemary"))
        chosen = self.window.settings_page._chosen_rosemary_program()
        quadra_programs = {
            str(program) for _name, program in rosemary_runtime.quadra_runners()
        }
        self.own_rosemary = bool(chosen) and chosen not in quadra_programs
        self.quadra_rosemary_row = Adw.SwitchRow(
            title=_("Use Quadra’s Rosemary"),
            subtitle=_(
                "Quadra downloads and updates its own build, which adds VR. "
                "Turn it off to use a Rosemary you installed yourself."
            ),
            active=not self.own_rosemary,
        )
        self.quadra_rosemary_row.connect(
            "notify::active", self._quadra_rosemary_toggled
        )
        group.add(self.quadra_rosemary_row)
        self.own_rosemary_row = Adw.ComboRow(
            title=_("Your Rosemary"), use_subtitle=False
        )
        self._own_rosemary_runners = []
        self._own_rosemary_handler = self.own_rosemary_row.connect(
            "notify::selected", self._own_rosemary_selected
        )
        group.add(self.own_rosemary_row)
        self.add_own_rosemary_row = Adw.ActionRow(
            title=_("Add your own Rosemary"),
            subtitle=_("Pick a rosemary program you built or downloaded"),
        )
        browse = Gtk.Button(label=_("Browse…"), valign=Gtk.Align.CENTER)
        browse.connect(
            "clicked",
            lambda *_args: self.window.settings_page._choose_rosemary_runner(
                self._show_rosemary_source
            ),
        )
        self.add_own_rosemary_row.add_suffix(browse)
        self.add_own_rosemary_row.set_activatable_widget(browse)
        group.add(self.add_own_rosemary_row)
        return group

    def _quadra_rosemary_toggled(self, row, _pspec):
        self.own_rosemary = not row.get_active()
        if not self.own_rosemary:
            self.window.set_setting("rosemary_program", "")
            self.window.set_setting("rosemary_source", "quadra")
        self._show_rosemary_source()

    def _own_rosemary_selected(self, row, _pspec):
        self.window.set_setting(
            "rosemary_program", self._own_rosemary_runners[row.get_selected()].program
        )
        self._show_rosemary_source()

    def _show_rosemary_source(self):
        page = self.window.settings_page
        runners = rosemary_runtime.user_runner_list(self.window.settings)
        programs = [runner.program for runner in runners]
        chosen = page._chosen_rosemary_program()
        if self.own_rosemary and chosen not in programs and programs:
            chosen = programs[0]
            self.window.set_setting("rosemary_program", chosen)
        with self.own_rosemary_row.handler_block(self._own_rosemary_handler):
            if runners != self._own_rosemary_runners:
                self._own_rosemary_runners = runners
                self.own_rosemary_row.set_model(
                    Gtk.StringList.new([runner.label for runner in runners])
                )
            if chosen in programs:
                self.own_rosemary_row.set_selected(programs.index(chosen))
        found = chosen in programs
        self.own_rosemary_row.set_subtitle(
            chosen
            if found
            else _(
                "None found. Install Rosemary with Butter or put it on PATH, or browse for it below."
            )
        )
        self.own_rosemary_row.set_visible(self.own_rosemary)
        self.add_own_rosemary_row.set_visible(self.own_rosemary)
        waiting_for_own = (
            self.window.rosemary_selected() and self.own_rosemary and not found
        )
        self.runtime_continue.set_sensitive(not waiting_for_own)

    def _darling_variant_selected(self, code):
        self.darling_variant = code
        self.window.set_setting("setup_darling_variant", code)

    @staticmethod
    def _step(number, title, description):
        row = Gtk.Box(spacing=14, css_classes=["card", "setup-step"])
        row.append(
            Gtk.Label(
                label=str(number), valign=Gtk.Align.START, css_classes=["setup-number"]
            )
        )
        text = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=5, hexpand=True)
        text.append(_label(title, classes=("heading",)))
        text.append(_label(description, classes=("dim-label",)))
        row.append(text)
        return row

    def _overview(self):
        page, body = self._page(
            "Here’s what happens next",
            "Everything you need to start playing, in one place.",
        )
        self.rosemary_steps = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        self.download_rosemary_step = self._step(
            1,
            "Download Rosemary",
            "Rosemary runs macOS apps on Linux. Quadra downloads it and checks it.",
        )
        self.rosemary_steps.append(self.download_rosemary_step)
        self.own_rosemary_step = self._step(
            1,
            "Use your Rosemary",
            "Rosemary runs macOS apps on Linux. Quadra runs "
            "the one you chose and leaves updating it to you.",
        )
        self.rosemary_steps.append(self.own_rosemary_step)
        self.bundled_rosemary_step = self._step(
            1,
            "Rosemary is included",
            "Rosemary runs macOS apps on Linux. "
            "Nix installs and updates it with Quadra.",
        )
        self.rosemary_steps.append(self.bundled_rosemary_step)
        self.rosemary_steps.append(
            self._step(
                2,
                "Download Roblox",
                "Get the official macOS client directly from Roblox.",
            )
        )
        self.rosemary_steps.append(
            self._step(
                3,
                "Make it yours",
                "Sign in to Roblox, then choose your graphics, "
                "mouse and launcher preferences in Settings.",
            )
        )
        body.append(self.rosemary_steps)
        self.darling_steps = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        self.darling_steps.append(
            self._step(
                1,
                "Install Darling",
                "A terminal opens to install Darling and the tools "
                "it needs. It may ask for your password. Quadra restarts when it is done.",
            )
        )
        self.darling_steps.append(
            self._step(
                2,
                "Download Roblox",
                "Get the official macOS client directly from Roblox, "
                "and prepare the files it needs in Darling.",
            )
        )
        self.darling_steps.append(
            self._step(
                3,
                "Make it yours",
                "Sign in to Roblox, then choose your graphics, "
                "mouse and launcher preferences in Settings.",
            )
        )
        body.append(self.darling_steps)

        folders = Gtk.Expander(label=_("Where your files go"))
        self.locations = Gtk.Box(
            orientation=Gtk.Orientation.VERTICAL,
            spacing=8,
            margin_top=10,
            margin_start=12,
            margin_end=12,
        )
        folders.set_child(self.locations)
        body.append(folders)
        self.overview_note = _label(
            "You can change launcher preferences at any time.",
            classes=("caption", "dim-label"),
            centered=True,
        )
        body.append(self.overview_note)
        actions, self.install_button = self._actions(
            "Install Roblox", self._start_install
        )
        body.append(actions)
        return page

    def _show_locations(self):
        while child := self.locations.get_first_child():
            self.locations.remove(child)
        if self.window.rosemary_selected():
            folders = (
                ("Rosemary and Roblox", user_directory("DATA")),
                (
                    "Rosemary’s app environment",
                    rosemary_runtime.prefix_path(self.window.settings),
                ),
            )
        else:
            folders = (
                ("Roblox and its downloads", core.DATA_DIR),
                ("Darling’s app environment", core.DARLING_PREFIX),
            )
        for title, path in (*folders, ("Launcher settings", core.CONFIG_DIR)):
            self.locations.append(_label(title, classes=("caption", "dim-label")))
            label = Gtk.Label(
                label=str(path),
                wrap=True,
                xalign=0,
                selectable=True,
                css_classes=["setup-path"],
            )
            label.set_wrap_mode(Pango.WrapMode.WORD_CHAR)
            self.locations.append(label)

    def _progress_page(self):
        page, body = self._page(
            "Getting Roblox ready",
            "This can take a few minutes. Keep Quadra open while setup finishes.",
            "folder-download-symbolic",
        )
        self.spinner = Gtk.Spinner(spinning=False, halign=Gtk.Align.CENTER)
        body.append(self.spinner)
        self.progress = Gtk.ProgressBar(css_classes=["setup-progress"], margin_top=8)
        body.append(self.progress)
        self.status = _label("Checking the latest Roblox version…", centered=True)
        body.append(self.status)
        self.error = Gtk.Label(
            wrap=True,
            xalign=0,
            selectable=True,
            visible=False,
            css_classes=["setup-note", "card"],
        )
        body.append(self.error)
        self.error_actions, self.retry_button = self._actions(
            "Try again",
            self._start_install,
            "Back",
            lambda *_args: self.show(self._previous_page("overview")),
        )
        self.error_actions.set_visible(False)
        body.append(self.error_actions)
        return page

    def _ready(self):
        page, body = self._page(
            "You’re ready to play",
            "Roblox is installed. Your next step is to open it and sign in.",
            "emblem-ok-symbolic",
        )
        note = Gtk.Box(
            orientation=Gtk.Orientation.VERTICAL,
            spacing=10,
            css_classes=["card", "setup-note"],
        )
        note.append(_label("Sign in inside Roblox", classes=("heading",)))
        note.append(
            _label(
                "Use your Roblox account on the sign-in screen. "
                "You don’t enter your password in this launcher."
            )
        )
        note.append(
            _label(
                "Quick Login is another option: get a code in Roblox, "
                "then enter it on a phone or browser where you’re already signed in.",
                classes=("dim-label",),
            )
        )
        body.append(note)
        actions, _button = self._actions(
            "Launch Roblox",
            lambda *_args: self.window.complete_setup(launch=True),
            "Open launcher",
            lambda *_args: self.window.complete_setup(),
        )
        body.append(actions)
        body.append(
            _label(
                "You can find this guide again in the launcher menu.",
                classes=("caption", "dim-label"),
                centered=True,
            )
        )
        return page

    def _page_after_appearance(self):
        return "overview" if self.rosemary_bundled else "runtime"

    def _previous_page(self, name):
        previous = PREVIOUS_PAGE[name]
        if previous == "runtime" and self.rosemary_bundled:
            return PREVIOUS_PAGE[previous]
        return previous

    def show(self, name):
        self.pages.set_visible_child_name(name)
        self.back.set_visible(name in PREVIOUS_PAGE)
        if name == "overview":
            rosemary = self.window.rosemary_selected()
            self.rosemary_steps.set_visible(rosemary)
            self.bundled_rosemary_step.set_visible(self.rosemary_bundled)
            self.download_rosemary_step.set_visible(
                not self.rosemary_bundled and not self.own_rosemary
            )
            self.own_rosemary_step.set_visible(
                not self.rosemary_bundled and self.own_rosemary
            )
            self.darling_steps.set_visible(not rosemary)
            self._show_locations()
            installed = self.window.client_installed()
            if installed:
                self.install_button.set_label(_("Continue"))
            elif rosemary or self._darling_ready():
                self.install_button.set_label(_("Install Roblox"))
            else:
                self.install_button.set_label(_("Install Darling"))
            self.overview_note.set_label(
                _("Roblox is already installed. Your settings and sign-in are kept.")
                if installed
                else _("You can change launcher preferences at any time.")
            )

    def _darling_ready(self):
        if darling_runtime.is_bundled():
            return True
        return (
            core.DARLING_RUNTIME["runtime"] == self.darling_variant
            and not core.missing_tools()
        )

    def continue_install(self):
        if self.window.rosemary_selected() or self._darling_ready():
            self._start_install()
            return False
        self.show("progress")
        self._finished(
            RuntimeError(
                _(
                    "Darling’s installer finished, but Quadra cannot find {runtime} with the "
                    "tools it needs. Check the terminal’s output, then try again.",
                    runtime=dict(
                        (code, _(title)) for code, title, _text in DARLING_VARIANTS
                    )[self.darling_variant],
                )
            )
        )
        return False

    def _start_install(self, *_args):
        if self.installing:
            return
        if self.window.client_installed() and not self.install_failed:
            self.show("ready")
            return
        self.installing = True
        self.show("progress")
        self.error.set_visible(False)
        self.error_actions.set_visible(False)
        self.progress.set_fraction(0)
        self.spinner.set_spinning(True)
        if self.window.rosemary_selected():
            self._install_rosemary()
        elif self._darling_ready():
            self._install_darling_client()
        else:
            self._install_darling()

    def _install_rosemary(self):
        uses_installed_rosemary = self.own_rosemary or self.rosemary_bundled
        self.status.set_label(
            _("Checking the latest Roblox version…")
            if uses_installed_rosemary
            else _("Downloading Rosemary…")
        )
        if not self.window.begin("installing Rosemary"):
            self._finished(
                RuntimeError(
                    _(
                        "Another operation is in progress. Wait for it to finish, then try again."
                    )
                )
            )
            return
        settings = self.window.settings.snapshot()
        needs_quadra_rosemary = (
            not uses_installed_rosemary and not rosemary_runtime.quadra_runners()
        )
        rosemary_share = 0.4 if needs_quadra_rosemary else 0.0

        def report(fraction, text):
            GLib.idle_add(self._progress, fraction, text)

        def report_client(fraction, text):
            report(rosemary_share + (1 - rosemary_share) * fraction, text)

        def install():
            error = None
            try:
                if needs_quadra_rosemary:
                    rosemary_install.install_latest(
                        lambda fraction, text: report(rosemary_share * fraction, text),
                        source="quadra",
                    )
                rosemary_runtime.ensure_ready(settings, report_client)
            except Exception as failure:
                error = failure
            GLib.idle_add(self._rosemary_installed, error)

        threading.Thread(target=install, daemon=True).start()

    def _rosemary_installed(self, error):
        self.window.end(resume_pending=False)
        self.window.client_changed()
        self._finished(error)
        return False

    def _install_darling(self):
        from .ui import system

        self.status.set_label(
            _(
                "Finish installing Darling in the terminal window. It may ask for your password."
            )
        )
        self.progress.pulse()
        release = (
            self.window.cider_release
            if self.darling_variant == "ciderdarling"
            else None
        )
        try:
            self.darling_job = runtime_install.Installation.create(
                core.PROJECT,
                self.darling_variant,
                core.CACHE_DIR,
                release=release,
                with_system_tools=True,
            )
            if not system._launch_runtime_terminal(
                self.window, self.darling_job.command
            ):
                raise RuntimeError(_("Could not open an installation terminal."))
        except (GLib.Error, OSError, RuntimeError, ValueError) as error:
            self._close_darling_job()
            self._finished(error)
            return
        GLib.timeout_add(1000, self._watch_darling_install)

    def _watch_darling_install(self):
        from .ui import system

        job = self.darling_job
        try:
            status = job.poll()
        except (OSError, RuntimeError, ValueError) as error:
            self._close_darling_job()
            self._finished(error)
            return False
        if status is None:
            self.progress.pulse()
            return True
        tail = job.log_tail(40)
        self._close_darling_job()
        if status != 0:
            message = _(
                "Darling’s installation failed or was cancelled. See the terminal for details."
            )
            if tail:
                message += "\n\n" + _("End of the build log:") + "\n" + tail
            self._finished(RuntimeError(message))
            return False
        self.status.set_label(_("Darling is installed. Restarting Quadra…"))
        self.window.settings_page.flush()
        try:
            system._restart_launcher("continue-setup")
        except OSError as error:
            self._finished(error)
        return False

    def _close_darling_job(self):
        if self.darling_job is not None:
            try:
                self.darling_job.close()
            except OSError:
                pass
            self.darling_job = None

    def _install_darling_client(self):
        self.status.set_label(_("Checking the latest Roblox version…"))
        missing = core.missing_tools()
        if missing:
            self._finished(
                RuntimeError(
                    _(
                        "Required programs are missing: {programs}. "
                        "Choose Darling again to install them, then try again.",
                        programs=", ".join(missing),
                    )
                )
            )
            return
        accepted = self.window.settings_page.check_updates(
            install=True,
            channel="",
            on_progress=self._progress,
            on_complete=self._finished,
        )
        if accepted is False:
            self._finished(
                RuntimeError(
                    _(
                        "Another operation is in progress. Wait for it to finish, then try again."
                    )
                )
            )

    def _progress(self, fraction, text):
        self.progress.set_fraction(max(0, min(1, fraction)))
        # Keep setup copy about the work the user is waiting for. Compiler and
        # framework details remain available in a failure's full error text.
        if fraction >= 0.97:
            text = _("Finishing setup…")
        elif fraction >= 0.94:
            text = _("Preparing Roblox to run on Linux…")
        elif fraction >= 0.9:
            text = _("Installing Roblox…")
        self.status.set_label(text)

    def _finished(self, error):
        self.installing = False
        self.spinner.set_spinning(False)
        if error:
            self.install_failed = True
            self.status.set_label(_("Setup couldn’t finish"))
            self.error.set_label(str(error) or repr(error))
            self.error.set_visible(True)
            self.error_actions.set_visible(True)
        else:
            self.install_failed = False
            self.progress.set_fraction(1)
            self.show("ready")
