"""Finding an X display that Darling can reach."""

import os
import socket
import subprocess
import time
from pathlib import Path

from ..i18n import _

__all__ = [
    "_find_working_x11_display",
    "_is_x11_reachable",
    "ensure_x11",
]


def _is_x11_reachable(display: str) -> bool:
    """Test whether an X11 server is reachable on the given DISPLAY string."""
    if not display:
        return False
    if display.startswith(":"):
        num = display[1:].split(".")[0]
        sock_path = f"/tmp/.X11-unix/X{num}"
        if not os.path.exists(sock_path):
            return False
        try:
            with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as s:
                s.settimeout(0.5)
                s.connect(sock_path)
                return True
        except OSError:
            return False
    try:
        host, port_str = display.split(":")
        port = 6000 + int(port_str.split(".")[0])
        with socket.create_connection((host or "127.0.0.1", port), timeout=0.5):
            return True
    except (OSError, ValueError):
        return False


def _find_working_x11_display() -> str | None:
    """Find any active local X11 display socket under /tmp/.X11-unix/."""
    socket_dir = Path("/tmp/.X11-unix")
    if not socket_dir.is_dir():
        return None
    for entry in socket_dir.glob("X*"):
        num = entry.name[1:]
        if num.isdigit():
            cand = f":{num}"
            if _is_x11_reachable(cand):
                return cand
    return None


def ensure_x11(env: dict[str, str]) -> None:
    """Ensure an X11 display is reachable, attempting to start Xwayland or recover if needed."""
    display = env.get("DISPLAY", ":0")
    if _is_x11_reachable(display):
        return

    # Check for another active X11 display socket
    alt = _find_working_x11_display()
    if alt:
        env["DISPLAY"] = alt
        return

    # In Wayland, try reviving user's Xwayland service (e.g. xwayland-satellite)
    if env.get("WAYLAND_DISPLAY"):
        try:
            res = subprocess.run(
                ["systemctl", "--user", "start", "xwayland-satellite.service"],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                timeout=3,
            )
            if res.returncode == 0:
                for _i in range(25):
                    time.sleep(0.1)
                    if _is_x11_reachable(display):
                        return
                    alt = _find_working_x11_display()
                    if alt:
                        env["DISPLAY"] = alt
                        return
        except Exception:
            pass

    # Final check
    alt = _find_working_x11_display()
    if alt:
        env["DISPLAY"] = alt
        return

    raise RuntimeError(
        _(
            "Cannot connect to X11 display {display}. Make sure an X server or Xwayland is running.",
            display=display,
        )
    )
