"""The CiderDarling desktop shortcut."""

import logging
import os
import subprocess
from pathlib import Path

from . import files, locations

__all__ = [
    "CIDERDARLING_SHORTCUT",
    "SHORTCUT_MARKER",
    "CIDERDARLING_DESKTOP",
    "_desktop_command",
    "sync_ciderdarling_shortcut",
]

# A shortcut to Settings -> CiderDarling while CiderDarling is installed; the
# same entry as packaging/org.quadrahq.Quadra.CiderDarling.desktop and the one
# launcher/install.sh writes. The marker line identifies a copy we may remove.
CIDERDARLING_SHORTCUT = "org.quadrahq.Quadra.CiderDarling.desktop"
SHORTCUT_MARKER = "X-QuadraHQ-Managed=true"
CIDERDARLING_DESKTOP = """[Desktop Entry]
Type=Application
Name=CiderDarling Settings
Name[ru]=Настройки CiderDarling
Comment=Graphics card, runtime and diagnostics settings for CiderDarling
Comment[ru]=Видеокарта, среда и диагностика для CiderDarling
Exec={command} --ciderdarling-settings
Icon=quadrahq
Terminal=false
Categories=Game;
Keywords=ciderdarling;darling;gpu;
StartupNotify=true
{marker}
"""


def _desktop_command():
    """How desktop entries start this launcher: the packages' wrapper (it sets
    the prebuilt shim path), or this checkout's script as a quoted argument."""
    if locations.PREBUILT_SHIM:
        return "quadrahq"
    text = str(locations.PROJECT / "launcher" / "quadrahq-launcher").replace("\\", "\\\\\\\\")
    for character in '"`$':
        text = text.replace(character, "\\\\" + character)
    return '"' + text.replace("%", "%%") + '"'


def sync_ciderdarling_shortcut(installed, applications=None):
    """Add the CiderDarling settings shortcut when CiderDarling is installed
    and the entry is missing, and remove our copy when it is not installed;
    True when the entry changed. An existing entry is left as it is: its
    command is launcher/install.sh's to update (another checkout may run)."""
    if os.environ.get("FLATPAK_ID"):
        return False
    if applications is None:
        data = os.environ.get("XDG_DATA_HOME") or str(Path.home() / ".local" / "share")
        applications = Path(data) / "applications"
    entry = Path(applications) / CIDERDARLING_SHORTCUT
    try:
        existing = (
            entry.read_text() if entry.is_file() and not entry.is_symlink() else None
        )
        ours = existing is not None and SHORTCUT_MARKER in existing.splitlines()
        if installed:
            if entry.exists() or entry.is_symlink():
                return False
            files._write_atomically(
                entry,
                CIDERDARLING_DESKTOP.format(
                    command=_desktop_command(), marker=SHORTCUT_MARKER
                ),
            )
        elif ours:
            entry.unlink()
        else:
            return False
    except OSError as error:
        logging.getLogger("quadra").warning(
            "Could not update the CiderDarling shortcut: %s", error
        )
        return False
    try:
        subprocess.run(
            ["update-desktop-database", str(applications)],
            stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            timeout=20,
        )
    except (OSError, subprocess.SubprocessError):
        pass
    return True
