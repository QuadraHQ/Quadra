"""The macOS Roblox client for Rosemary: find, download, unpack, verify.

Adapted from nutmeg (https://github.com/frolleks/nutmeg, GPL-3.0-or-later,
Copyright frolleks), nutmeg/roblox.py, for Quadra's directories and errors.
Kept apart from the Darling client (core.APP_BUNDLE): Quadra patches that
bundle for Darling, and Rosemary runs an untouched copy.

Roblox publishes the current macOS build at
https://clientsettingscdn.roblox.com/v2/client-version/MacPlayer as
{"version": "0.741.0.7411056", "clientVersionUpload": "version-<hex>"} and the
build as a zip holding RobloxPlayer.app on setup.rbxcdn.com (mac/ for x86_64,
mac/arm64/ for arm64). Each zip has a thin executable for its architecture.
"""

import http.client
import json
import os
import platform
import plistlib
import re
import shutil
import stat
import tempfile
import threading
import urllib.error
import urllib.request
import zipfile
import zlib
from dataclasses import dataclass
from pathlib import Path

from .paths import user_directory

TIMEOUT = 30
CLIENT_VERSION_URLS = (
    "https://clientsettingscdn.roblox.com/v2/client-version/MacPlayer",
    "https://clientsettings.roblox.com/v2/client-version/MacPlayer",
)
CDN_HOSTS = ("https://setup.rbxcdn.com", "https://setup-aws.rbxcdn.com", "https://setup-ak.rbxcdn.com")
BUNDLE_IN_ZIP = "RobloxPlayer.app"
BUNDLE_NAME = "Roblox.app"
CPU_TYPES = {"x86_64": 0x01000007, "arm64": 0x0100000C}
_GUID = re.compile(r"^version-[0-9a-f]+$")
_VERSION = re.compile(r"^[0-9]+(\.[0-9]+)*$")
_SPARE_BYTES = 64 * 1024 * 1024
_CHUNK = 1024 * 1024


class ClientError(Exception):
    """A problem worded for the person using Quadra."""


class Cancelled(Exception):
    pass


def data_dir():
    return user_directory("DATA") / "rosemary"


def versions_dir():
    return data_dir() / "versions"


def downloads_dir():
    return data_dir() / "downloads"


def host_arch():
    """This machine's architecture as Roblox's builds name it, or None."""
    machine = platform.machine().lower()
    if machine in ("x86_64", "amd64"):
        return "x86_64"
    if machine in ("aarch64", "arm64"):
        return "arm64"
    return None


@dataclass(frozen=True)
class Build:
    guid: str      # `version-<hex>`, names the build on the CDN
    version: str   # dotted version for display, "" when unknown
    arch: str      # x86_64 or arm64

    @property
    def key(self):
        return f"{self.guid}-{self.arch}"

    @property
    def label(self):
        return self.version or self.guid

    def directory(self):
        return versions_dir() / self.key

    def bundle(self):
        return self.directory() / BUNDLE_NAME

    def zip_urls(self):
        folder = "mac/arm64" if self.arch == "arm64" else "mac"
        return [f"{host}/{folder}/{self.guid}-RobloxPlayer.zip" for host in CDN_HOSTS]


def _open(url, opener=urllib.request.urlopen):
    return opener(urllib.request.Request(url, headers={"User-Agent": "Quadra"}), timeout=TIMEOUT)


def _describe(error):
    if isinstance(error, urllib.error.HTTPError):
        return f"HTTP {error.code}"
    if isinstance(error, urllib.error.URLError):
        return str(error.reason)
    return str(error) or type(error).__name__


def fetch_latest(arch, opener=urllib.request.urlopen):
    """The build Roblox currently ships for macOS."""
    failures = []
    for url in CLIENT_VERSION_URLS:
        try:
            with _open(url, opener) as response:
                data = json.loads(response.read(1024 * 1024).decode("utf-8"))
        except (OSError, ValueError, http.client.HTTPException) as error:
            failures.append(f"{url}: {_describe(error)}")
            continue
        guid = data.get("clientVersionUpload") if isinstance(data, dict) else None
        version = data.get("version") if isinstance(data, dict) else None
        if not isinstance(guid, str) or not _GUID.match(guid):
            failures.append(f"{url}: unexpected answer {str(data)[:200]!r}")
            continue
        if not isinstance(version, str) or not _VERSION.match(version):
            version = ""
        return Build(guid=guid, version=version, arch=arch)
    raise ClientError("Could not find out the current Roblox version.\n" + "\n".join(failures))


def _check_space(directory, needed):
    free = shutil.disk_usage(directory).free
    if free < needed + _SPARE_BYTES:
        raise ClientError(f"Not enough disk space in {directory}: {needed / 1e6:.0f} MB is needed, "
                          f"{free / 1e6:.0f} MB is free.")


def _check_zip(path):
    with zipfile.ZipFile(path) as archive:
        if f"{BUNDLE_IN_ZIP}/Contents/Info.plist" not in archive.namelist():
            raise zipfile.BadZipFile(f"the archive has no {BUNDLE_IN_ZIP}")


def download(build, progress, cancel, opener=urllib.request.urlopen):
    """Download the build's zip, trying each CDN host in turn.
    progress(done_bytes, total_bytes_or_None)."""
    directory = downloads_dir()
    directory.mkdir(parents=True, exist_ok=True)
    target = directory / f"{build.key}-RobloxPlayer.zip"
    partial = target.with_name(target.name + ".part")
    failures = []
    for url in build.zip_urls():
        if cancel.is_set():
            raise Cancelled()
        try:
            with _open(url, opener) as response, open(partial, "wb") as out:
                length = response.headers.get("Content-Length")
                total = int(length) if length and length.isdigit() else None
                if total is not None:
                    _check_space(directory, total)
                done = 0
                progress(done, total)
                while True:
                    if cancel.is_set():
                        raise Cancelled()
                    chunk = response.read(_CHUNK)
                    if not chunk:
                        break
                    out.write(chunk)
                    done += len(chunk)
                    progress(done, total)
            if total is not None and done != total:
                raise OSError(f"the download stopped at {done} of {total} bytes")
            _check_zip(partial)
        except (OSError, ValueError, http.client.HTTPException, zipfile.BadZipFile) as error:
            partial.unlink(missing_ok=True)
            failures.append(f"{url}: {_describe(error)}")
            continue
        except BaseException:
            partial.unlink(missing_ok=True)
            raise
        os.replace(partial, target)
        return target
    raise ClientError(f"Could not download Roblox {build.label}.\n" + "\n".join(failures))


def _member_parts(name):
    """Path components of an archive member; refuses any that would land
    outside the directory being extracted into."""
    if name.startswith("/") or "\0" in name:
        raise ClientError(f"The Roblox archive holds an unsafe path: {name!r}")
    parts = [part for part in name.split("/") if part not in ("", ".")]
    if not parts or ".." in parts or parts[0] != BUNDLE_IN_ZIP:
        raise ClientError(f"The Roblox archive holds an unexpected path: {name!r}")
    return parts


def _check_link_target(name, target):
    # A relative target without `..` can only point at or below the directory
    # holding the link, which is inside the extraction root, so no chain of
    # links escapes.
    if not target or target.startswith("/") or "\0" in target or ".." in target.split("/"):
        raise ClientError(f"The Roblox archive holds an unsafe link: {name!r} -> {target!r}")


def _extract_member(archive, member, root):
    destination = root.joinpath(*_member_parts(member.filename))
    mode = member.external_attr >> 16 if member.create_system == 3 else 0
    kind = stat.S_IFMT(mode)
    if member.is_dir() or kind == stat.S_IFDIR:
        destination.mkdir(parents=True, exist_ok=True)
        return
    destination.parent.mkdir(parents=True, exist_ok=True)
    if kind == stat.S_IFLNK:
        try:
            target = archive.read(member).decode("utf-8")
        except UnicodeDecodeError as error:
            raise ClientError(f"The Roblox archive holds an unreadable link: {member.filename!r}") from error
        _check_link_target(member.filename, target)
        os.symlink(target, destination)
        return
    if kind not in (0, stat.S_IFREG):
        raise ClientError(f"The Roblox archive holds a special file: {member.filename!r}")
    permissions = 0o777 if mode & 0o111 else 0o666  # only executability is kept
    fd = os.open(destination, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW | os.O_CLOEXEC, permissions)
    with os.fdopen(fd, "wb") as out, archive.open(member) as source:
        shutil.copyfileobj(source, out, _CHUNK)  # reading to the end checks the CRC


def executable(bundle):
    """The bundle's main executable, by its Info.plist."""
    info = bundle / "Contents" / "Info.plist"
    try:
        with open(info, "rb") as handle:
            name = plistlib.load(handle).get("CFBundleExecutable")
    except Exception as error:
        raise ClientError(f"Could not read {info}: {error}") from error
    if not isinstance(name, str) or not name or "/" in name or name.startswith("."):
        raise ClientError(f"{info} names no usable executable")
    return bundle / "Contents" / "MacOS" / name


def macho_architectures(path):
    """Architectures a Mach-O file has code for (CPU_TYPES names); empty for
    anything that is not a 64-bit Mach-O or fat file."""
    names = {cpu: name for name, cpu in CPU_TYPES.items()}
    with open(path, "rb") as handle:
        head = handle.read(8)
        if len(head) < 8:
            return set()
        if int.from_bytes(head[:4], "little") == 0xFEEDFACF:
            cpu = int.from_bytes(head[4:8], "little")
            return {names[cpu]} if cpu in names else set()
        magic = int.from_bytes(head[:4], "big")
        if magic not in (0xCAFEBABE, 0xCAFEBABF):
            return set()
        count = int.from_bytes(head[4:8], "big")
        if count > 16:  # a Java class file shares the magic
            return set()
        size = 20 if magic == 0xCAFEBABE else 32
        table = handle.read(count * size)
    return {names[cpu] for i in range(min(count, len(table) // size))
            if (cpu := int.from_bytes(table[i * size:i * size + 4], "big")) in names}


def _verify_bundle(bundle, arch):
    program = executable(bundle)
    if not program.is_file() or not os.access(program, os.X_OK):
        raise ClientError(f"{program} is missing or not executable")
    found = macho_architectures(program)
    if arch not in found:
        raise ClientError(f"{program} was expected to be {arch}, but has "
                          f"{', '.join(sorted(found)) or 'no recognised architecture'}")
    return program


def install(zip_path, build, progress, cancel):
    """Unpack a downloaded zip into the build's directory (complete or not at
    all) and return the executable. progress(done_members, total_members)."""
    versions = versions_dir()
    versions.mkdir(parents=True, exist_ok=True)
    staging = Path(tempfile.mkdtemp(prefix=f".tmp-{build.key}-", dir=versions))
    try:
        with zipfile.ZipFile(zip_path) as archive:
            members = archive.infolist()
            _check_space(versions, sum(m.file_size for m in members))
            for index, member in enumerate(members, 1):
                if cancel.is_set():
                    raise Cancelled()
                _extract_member(archive, member, staging)
                progress(index, len(members))
        unpacked = staging / BUNDLE_IN_ZIP
        _verify_bundle(unpacked, build.arch)
        os.rename(unpacked, staging / BUNDLE_NAME)
        final = build.directory()
        if final.exists():
            shutil.rmtree(final)  # installed earlier but no longer verifies
        os.rename(staging, final)
    except (zipfile.BadZipFile, zlib.error, EOFError) as error:
        shutil.rmtree(staging, ignore_errors=True)
        raise ClientError(f"The downloaded Roblox archive is damaged: {error}") from error
    except BaseException:
        shutil.rmtree(staging, ignore_errors=True)
        raise
    return _verify_bundle(build.bundle(), build.arch)


def installed_builds():
    """[(Build, bundle_path)] of intact installed builds, newest first by mtime."""
    found = []
    try:
        entries = [d for d in versions_dir().iterdir() if d.is_dir() and not d.name.startswith(".")]
    except OSError:
        return []
    for directory in sorted(entries, key=lambda d: d.stat().st_mtime, reverse=True):
        guid, _, arch = directory.name.rpartition("-")
        if not _GUID.match(guid) or arch not in CPU_TYPES:
            continue
        bundle = directory / BUNDLE_NAME
        try:
            _verify_bundle(bundle, arch)
            with open(bundle / "Contents" / "Info.plist", "rb") as handle:
                version = plistlib.load(handle).get("CFBundleShortVersionString")
        except Exception:
            continue
        found.append((Build(guid, version if isinstance(version, str) else "", arch), bundle))
    return found


def remove_old(keep_key, in_use=(), keep_previous=1, pinned=None):
    """Delete installed builds other than `keep_key`, `pinned` and `in_use`,
    keeping the newest `keep_previous` of them so there is a build to go back to."""
    others = [d for d in versions_dir().glob("version-*")
              if d.is_dir() and d.name not in (keep_key, pinned) and d.name not in in_use]
    others.sort(key=lambda d: d.stat().st_mtime, reverse=True)
    for directory in others[keep_previous:]:
        shutil.rmtree(directory, ignore_errors=True)


def write_fast_flags(bundle, flags):
    """Write the shared Fast flags into this bundle's ClientAppSettings.json."""
    target = bundle / "Contents" / "MacOS" / "ClientSettings" / "ClientAppSettings.json"
    target.parent.mkdir(parents=True, exist_ok=True)
    temporary = target.with_suffix(".tmp")
    temporary.write_text(json.dumps(flags, indent=2))
    os.replace(temporary, target)
