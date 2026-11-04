"""The saved Roblox sign-in: whether there is one, and signing out."""

import plistlib
import shutil
import subprocess

from . import darling_server, locations, process_control

__all__ = [
    "_darling_path",
    "logout",
    "signed_in",
]


def _darling_path(path):
    """Path of a file under the Darling prefix as seen inside the container."""
    return "/" + str(path.relative_to(locations.DARLING_PREFIX))


def signed_in():
    """Whether a Roblox login is saved (only the cookie's name is read)."""
    try:
        with open(locations.SESSION_FILES[0], "rb") as file:
            cookies = plistlib.load(file)
    except Exception:  # missing or damaged (plistlib raises several kinds)
        return False
    return any(
        isinstance(c, dict) and c.get("Name") == ".ROBLOSECURITY" and c.get("Value")
        for c in cookies
        if isinstance(cookies, list)
    )


def logout():
    """Delete the saved Roblox session. Returns True when it is gone.
    Blocks up to a minute: call it off the GTK thread."""
    shutil.rmtree(locations.WEB_DATA_DIR, ignore_errors=True)
    # Files inside ~/.darling must not be removed from the host while
    # darlingserver runs: its overlay then stops showing new files to the host.
    if process_control.darlingserver_running():
        try:
            subprocess.run(
                [
                    locations.DARLING_EXECUTABLE,
                    "shell",
                    "/bin/rm",
                    "-rf",
                    *[_darling_path(path) for path in locations.SESSION_FILES],
                ],
                env=darling_server.darling_environment(),
                stdin=subprocess.DEVNULL,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                timeout=60,
            )
        except (OSError, subprocess.SubprocessError):
            pass
    else:
        for path in locations.SESSION_FILES:
            try:
                if path.is_dir():
                    shutil.rmtree(path)
                elif path.exists():
                    path.unlink()
            except OSError:
                pass
    return not any(path.exists() for path in locations.SESSION_FILES)
