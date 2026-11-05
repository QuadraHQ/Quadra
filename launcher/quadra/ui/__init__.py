"""GTK 4 / libadwaita interface of the Quadra launcher.

The interface only shows things and asks quadra.core to do them. Its modules,
in the order they load (a module only uses the ones above it):

    dialogs          toasts, the copyable error dialog, the VR warning
    widgets          small widgets and layout helpers shared by the pages
    system           restarting the launcher, opening a terminal, the project's links
    logs_view        the game log viewer on the Play page
    play_page        Play
    flags_page       the fast flag editors (client and Studio)
    studio_page      Roblox Studio
    settings_page    Settings and its tabs, with the runtime choice
    info_page        Info
    mods_page        Mods
    servers_page     Servers
    boot_splash      the splash shown while Roblox starts
    launcher_window  the window: holds the pages, starts, watches and stops the game
    launcher_app     the GTK application and main()

The first-launch setup wizard is quadra/setup.py. `ui.NAME` reaches every name
of these modules, and setting or patching it changes it in the module that
owns it (see quadra/namespace.py).
"""

from typing import TYPE_CHECKING

import gi

# Before any module below imports GTK.
gi.require_version("Gtk", "4.0")
gi.require_version("Adw", "1")
try:
    gi.require_version("GLibUnix", "2.0")
except ValueError:
    pass  # GLib older than 2.80: launcher_app uses GLib.unix_signal_add

from .. import namespace
from . import (
    boot_splash,
    dialogs,
    flags_page,
    info_page,
    launcher_app,
    launcher_window,
    logs_view,
    mods_page,
    play_page,
    servers_page,
    settings_page,
    studio_page,
    system,
    widgets,
)

namespace.share_names(
    __name__,
    [
        dialogs,
        widgets,
        system,
        logs_view,
        play_page,
        flags_page,
        studio_page,
        settings_page,
        info_page,
        mods_page,
        servers_page,
        boot_splash,
        launcher_window,
        launcher_app,
    ],
)

if TYPE_CHECKING:  # tells editors and type checkers what ui.NAME is
    from .boot_splash import *
    from .dialogs import *
    from .flags_page import *
    from .info_page import *
    from .launcher_app import *
    from .launcher_window import *
    from .logs_view import *
    from .mods_page import *
    from .play_page import *
    from .servers_page import *
    from .settings_page import *
    from .studio_page import *
    from .system import *
    from .widgets import *
