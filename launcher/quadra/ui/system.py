"""Leaving the launcher: restarting it, opening a terminal, and the project's links."""

import os
import subprocess
import sys

from gi.repository import Gio, GLib, Gtk

from .. import author, core, terminal
from ..i18n import _

__all__ = [
    "_restart_launcher",
    "_launch_runtime_terminal",
    "_links",
    "_open_uri",
]


def _restart_launcher(page="settings"):
    # Start the normal entry point without replaying an original Roblox URI.
    # A fresh process loads the selected client, sysroot and prefix together.
    environment = dict(os.environ, QUADRA_PAGE=page)
    entry = core.PROJECT / "launcher/quadrahq-launcher"
    if not entry.is_file():
        raise FileNotFoundError(
            _(
                "Quadra's launcher files are missing. Reinstall Quadra before restarting."
            )
        )
    os.execve(sys.executable, [sys.executable, str(entry)], environment)


def _launch_runtime_terminal(window, command):
    argv = terminal.command(command)
    if argv:
        subprocess.Popen(argv, start_new_session=True)
        return True
    # GIO's own terminal list is the last resort. Exec field codes require
    # literal '%' to be doubled even inside quoted script text or filenames.
    line = " ".join(GLib.shell_quote(argument) for argument in command).replace(
        "%", "%%"
    )
    application = Gio.AppInfo.create_from_commandline(
        line, _("Install CiderDarling"), Gio.AppInfoCreateFlags.NEEDS_TERMINAL
    )
    try:
        return application.launch([], window.get_display().get_app_launch_context())
    except GLib.Error as error:
        raise RuntimeError(
            _(
                "No terminal application was found. Install one (for example xterm), "
                "or set the TERMINAL environment variable to your terminal's command, "
                "then try again."
            )
            + f"\n\n{error.message}"
        ) from error


def _links():
    """(title, icon, uri) of the project's community pages."""
    links = [
        ("Website", "quadra-web-symbolic", "https://quadrahq.org"),
        ("Discord", "quadra-discord-symbolic", author.DISCORD_URL),
    ]
    if author.GITHUB_URL:
        links.append(("GitHub", "quadra-github-symbolic", author.GITHUB_URL))
    return links


def _open_uri(window, uri):
    Gtk.UriLauncher.new(uri).launch(window, None, None, None)
