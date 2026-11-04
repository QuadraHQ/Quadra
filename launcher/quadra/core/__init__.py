"""The launcher's core: installing, preparing and running Roblox. No GTK here.

Each topic has its own module, listed in the order they load (a module only
uses the ones above it):

    locations        where things are: folders, the Roblox bundle, the Darling runtime and prefix
    files            writing files safely
    config           settings: defaults, loading and saving, the last error
    fast_flags       the user's fast flags and the ones Darling needs
    logs             reading the client's logs, keeping the logs folder small
    roblox           the Roblox client: version, updates, patches, deleting it
    builds           building the shim and Quadra M2VK, and the programs that needs
    launcher_update  updating the launcher itself
    desktop          the CiderDarling desktop shortcut
    process_control  finding, stopping and prioritising Roblox and Darling processes
    darling_server   the Darling container: environment, server start and restart
    account          the saved Roblox sign-in, and signing out
    x11              finding an X display Darling can reach
    prefix_files     the files Quadra installs into the Darling prefix
    host             what the client is told about the host (icon, GPU memory)
    audio            playing the client's sound on the host
    session          RobloxSession: one run of the client
    server_location  the game server's address and location

`core.NAME` reaches every name of these modules, and setting or patching it
changes it in the module that owns it (see quadra/namespace.py).
"""
from typing import TYPE_CHECKING

from .. import namespace
from . import (
    locations,
    files,
    config,
    state,
    fast_flags,
    logs,
    roblox,
    builds,
    launcher_update,
    desktop,
    process_control,
    darling_server,
    account,
    x11,
    prefix_files,
    host,
    audio,
    session,
    server_location,
)

namespace.share_names(__name__, [
    locations, files, config, state, fast_flags, logs, roblox, builds, launcher_update, desktop,
    process_control, darling_server, account, x11, prefix_files, host, audio, session, server_location,
])

if TYPE_CHECKING:  # tells editors and type checkers what core.NAME is
    from .locations import *  # noqa: F403
    from .files import *  # noqa: F403
    from .config import *  # noqa: F403
    from .state import *  # noqa: F403
    from .fast_flags import *  # noqa: F403
    from .logs import *  # noqa: F403
    from .roblox import *  # noqa: F403
    from .builds import *  # noqa: F403
    from .launcher_update import *  # noqa: F403
    from .desktop import *  # noqa: F403
    from .process_control import *  # noqa: F403
    from .darling_server import *  # noqa: F403
    from .account import *  # noqa: F403
    from .x11 import *  # noqa: F403
    from .prefix_files import *  # noqa: F403
    from .host import *  # noqa: F403
    from .audio import *  # noqa: F403
    from .session import *  # noqa: F403
    from .server_location import *  # noqa: F403
