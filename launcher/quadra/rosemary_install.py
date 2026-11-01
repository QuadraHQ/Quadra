"""Install Rosemary from its official release (GitHub: microlift/rosemary).

Downloads the x86_64 AppImage and SHA256SUMS of the latest release, checks the
file against the sums, and unpacks it with `--appimage-extract` into
<data>/rosemary-runner/<version>/squashfs-root, where rosemary_runtime finds
it. The AppImage is deleted afterwards (the unpacked copy runs without FUSE).
Nothing outside that folder is touched."""

import hashlib
import json
import os
import re
import shutil
import subprocess
import urllib.parse
import urllib.request
from pathlib import Path

from .paths import user_directory

# Where Rosemary comes from, in order: Quadra's own fork (the VR and
# injection work, once it publishes a release) and then upstream. A source
# with no usable release is skipped. The suffix keeps their folders apart.
SOURCES = (("QuadraHQ/rosemary-fork", "quadra"), ("microlift/rosemary", ""))
ALLOWED_HOSTS = ("api.github.com", "github.com", "objects.githubusercontent.com", "release-assets.githubusercontent.com")
APPIMAGE = re.compile(r"^Rosemary-([0-9][0-9A-Za-z.\-]*)-x86_64\.AppImage$")
VERSION_OK = re.compile(r"^[0-9][0-9A-Za-z.\-]*$")
MAX_APPIMAGE = 600 * 1024 * 1024


class InstallError(Exception):
    pass


def runner_root():
    return user_directory("DATA") / "rosemary-runner"


def _check_url(url):
    parts = urllib.parse.urlsplit(url)
    if parts.scheme != "https" or parts.hostname not in ALLOWED_HOSTS:
        raise InstallError(f"Refusing to download from {url}")


def _get(url, opener=urllib.request.urlopen, limit=4 * 1024 * 1024):
    _check_url(url)
    with opener(urllib.request.Request(url, headers={"User-Agent": "Quadra"}), timeout=30) as response:
        return response.read(limit)


def parse_release(data):
    """(version, appimage_name, appimage_url, sums_url) from the release JSON."""
    try:
        assets = {a["name"]: a["browser_download_url"] for a in data["assets"]}
        for name, url in assets.items():
            match = APPIMAGE.match(name)
            if match and "SHA256SUMS" in assets:
                return match.group(1), name, url, assets["SHA256SUMS"]
    except (KeyError, TypeError):
        pass
    raise InstallError("The latest Rosemary release has no x86_64 AppImage with SHA256SUMS.")


def expected_hash(sums_text, name):
    for line in sums_text.splitlines():
        parts = line.split()
        if len(parts) == 2 and parts[1].lstrip("*") == name and re.fullmatch(r"[0-9a-f]{64}", parts[0]):
            return parts[0]
    raise InstallError(f"SHA256SUMS has no entry for {name}.")


def find_release(opener=urllib.request.urlopen, sources=SOURCES):
    """(version, folder name, appimage name, appimage url, sums url) of the
    newest release of the first source that has one."""
    failures = []
    for repo, suffix in sources:
        try:
            data = json.loads(_get(f"https://api.github.com/repos/{repo}/releases/latest", opener))
            version, name, url, sums_url = parse_release(data)
        except (OSError, ValueError, InstallError) as error:
            failures.append(f"{repo}: {error}")
            continue
        if not VERSION_OK.match(version):
            failures.append(f"{repo}: unexpected version {version!r}")
            continue
        return version, (f"{version}-{suffix}" if suffix else version), name, url, sums_url
    raise InstallError("Could not find a Rosemary release to install.\n" + "\n".join(failures))


def latest_version(opener=urllib.request.urlopen):
    """The folder name the newest release would install into, or None offline."""
    try:
        return find_release(opener)[1]
    except InstallError:
        return None


def install_latest(progress, opener=urllib.request.urlopen):
    """Install the newest release. progress(fraction, text). Returns the
    executable path. Blocking: call off the GTK thread."""
    progress(0.0, "Finding the latest Rosemary")
    version, folder, name, url, sums_url = find_release(opener)
    target = runner_root() / folder
    program = target / "squashfs-root" / "bin" / "rosemary"
    if program.is_file() and os.access(program, os.X_OK):
        progress(1.0, f"Rosemary {version} is installed")
        return program
    wanted = expected_hash(_get(sums_url, opener).decode("utf-8", "replace"), name)
    staging = runner_root() / f".installing-{folder}"
    shutil.rmtree(staging, ignore_errors=True)
    staging.mkdir(parents=True)
    try:
        image = staging / name
        _check_url(url)
        digest = hashlib.sha256()
        with opener(urllib.request.Request(url, headers={"User-Agent": "Quadra"}), timeout=60) as response, \
                open(image, "wb") as out:
            length = response.headers.get("Content-Length")
            total = int(length) if length and length.isdigit() else None
            done = 0
            while chunk := response.read(1 << 20):
                done += len(chunk)
                if done > MAX_APPIMAGE:
                    raise InstallError("The Rosemary download is larger than expected.")
                digest.update(chunk)
                out.write(chunk)
                if total:
                    progress(0.8 * done / total, f"Downloading Rosemary {version}")
        if digest.hexdigest() != wanted:
            raise InstallError("The downloaded Rosemary does not match its SHA256SUMS; not installing it.")
        image.chmod(0o755)
        progress(0.82, "Unpacking Rosemary")
        result = subprocess.run([str(image), "--appimage-extract"], cwd=staging, stdin=subprocess.DEVNULL,
                                capture_output=True, text=True, timeout=300)
        unpacked = staging / "squashfs-root" / "bin" / "rosemary"
        if result.returncode != 0 or not unpacked.is_file():
            raise InstallError("Could not unpack the Rosemary AppImage:\n" + (result.stderr or result.stdout)[-400:])
        check = subprocess.run([str(unpacked), "--version"], stdin=subprocess.DEVNULL, capture_output=True,
                               text=True, timeout=30)
        if check.returncode != 0:
            raise InstallError("The unpacked Rosemary does not start:\n" + (check.stderr or check.stdout)[-400:])
        image.unlink()
        if target.exists():
            shutil.rmtree(target)
        os.rename(staging, target)
    except BaseException:
        shutil.rmtree(staging, ignore_errors=True)
        raise
    progress(1.0, f"Rosemary {version} installed")
    return program
