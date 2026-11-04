"""Updating the launcher itself from its git checkout or GitHub release."""

import json
import re
import subprocess
import urllib.request

from .. import __version__
from ..i18n import _
from . import builds, config, locations

__all__ = [
    "LAUNCHER_RELEASE_URL",
    "LAUNCHER_SPARSE_PATTERNS",
    "RELEASES_URL",
    "REPO_URL",
    "_git",
    "check_launcher_update",
    "parse_version_tuple",
    "update_launcher",
]

REPO_URL = "https://github.com/QuadraHQ/Quadra.git"
RELEASES_URL = "https://github.com/QuadraHQ/Quadra/releases/latest"
LAUNCHER_RELEASE_URL = "https://api.github.com/repos/QuadraHQ/Quadra/releases/latest"
# Keep these patterns aligned with install.sh. Hosting files stay on main,
# while partial app fetches only materialize blobs selected by this checkout.
LAUNCHER_SPARSE_PATTERNS = """/*
!/website/
!/.github/
!/vercel.json
!/.vercel/
!/.vercelignore
"""


def parse_version_tuple(ver):
    return tuple(int(x) for x in re.findall(r"\d+", ver)) if ver else (0,)


def check_launcher_update():
    """Checks GitHub for a newer release of Quadra.
    Returns (has_update, latest_version_string, release_url); raises when
    GitHub cannot be reached, so a failed check is not reported as up to date."""
    req = urllib.request.Request(LAUNCHER_RELEASE_URL, headers={"User-Agent": "Quadra"})
    with urllib.request.urlopen(req, timeout=6) as response:
        data = json.loads(response.read().decode())
    tag = str(data.get("tag_name") or "").lstrip("v")
    html_url = data.get("html_url") or RELEASES_URL
    has_update = bool(tag) and parse_version_tuple(tag) > parse_version_tuple(
        __version__
    )
    return has_update, tag, html_url


def _git(*args, input=None):
    return subprocess.run(
        ["git", "-C", str(locations.PROJECT), *args],
        input=input,
        capture_output=True,
        text=True,
    )


def update_launcher(progress=None):
    """Updates Quadra the way install.sh does: pulls the checkout
    (fast-forward only, so a failed update never leaves a half merge),
    rebuilds Darling's shim when Darling is chosen and reinstalls the menu
    entries. Returns (True,
    message), or (False, release page) when this is not a git checkout
    (Flatpak, packages)."""
    if not (locations.PROJECT / ".git").is_dir():
        return False, RELEASES_URL
    if progress:
        progress(0.2, _("Pulling latest version…"))
    # Match the installer's migration of recognized legacy repository URLs.
    origin = _git("remote", "get-url", "origin").stdout.strip()
    legacy_remotes = {
        "https://github.com/narezy/MacOBlox",
        "https://github.com/narezy/MacOBlox.git",
        "https://github.com/aubree-lat/MacOBlox",
        "https://github.com/aubree-lat/MacOBlox.git",
        "git@github.com:aubree-lat/MacOBlox",
        "git@github.com:aubree-lat/MacOBlox.git",
        "ssh://git@github.com/aubree-lat/MacOBlox",
        "ssh://git@github.com/aubree-lat/MacOBlox.git",
        "https://github.com/CiderHQ-org/Cider",
        "https://github.com/CiderHQ-org/Cider.git",
        "git@github.com:CiderHQ-org/Cider",
        "git@github.com:CiderHQ-org/Cider.git",
        "ssh://git@github.com/CiderHQ-org/Cider",
        "ssh://git@github.com/CiderHQ-org/Cider.git",
    }
    if origin in legacy_remotes:
        result = _git("remote", "set-url", "origin", REPO_URL)
        if result.returncode:
            raise RuntimeError(
                _(
                    "Could not update the files in {path}:\n{output}",
                    path=locations.PROJECT,
                    output=(result.stderr or result.stdout).strip(),
                )
            )
    for arguments, patterns in (
        (("config", "remote.origin.promisor", "true"), None),
        (("config", "remote.origin.partialclonefilter", "blob:none"), None),
        (("sparse-checkout", "set", "--no-cone", "--stdin"), LAUNCHER_SPARSE_PATTERNS),
        (("fetch", "--filter=blob:none", "origin", "main"), None),
        (("merge", "--ff-only", "origin/main"), None),
    ):
        result = _git(*arguments, input=patterns)
        if result.returncode != 0:
            raise RuntimeError(
                _(
                    "Could not update the files in {path}:\n{output}",
                    path=locations.PROJECT,
                    output=(result.stderr or result.stdout).strip(),
                )
            )

    if config.load_settings().get("runtime_backend") == "darling":
        if progress:
            progress(0.6, _("Building shim…"))
        ok, output = builds.build_shim()
        if not ok:
            raise RuntimeError(_("Could not build the shim:\n{output}", output=output))

    if progress:
        progress(0.9, _("Updating launcher shortcuts…"))
    install_script = locations.PROJECT / "launcher" / "install.sh"
    if install_script.exists():
        subprocess.run(
            [str(install_script)],
            cwd=str(locations.PROJECT),
            check=True,
            capture_output=True,
        )

    if progress:
        progress(1.0, _("Quadra updated successfully"))
    # This process keeps running the code it started with.
    return True, _("Quadra updated. Restart it to use the new version.")
