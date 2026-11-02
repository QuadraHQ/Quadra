"""The Roblox client: its version, checking for and installing updates, the patches applied
to it, and deleting it."""

import hashlib
import json
import logging
import plistlib
import re
import shutil
import subprocess
import sys
import tempfile
import urllib.error
import zipfile
from pathlib import Path

from ..i18n import _
from ..roblox_downloads import open_public
from . import config, fast_flags, locations, logs

__all__ = [
    "installed_version",
    "_validated_deployment",
    "version_is_newer",
    "latest_version",
    "update_roblox",
    "throttle_patch_state",
    "run_throttle_patcher",
    "remove_throttle_patch",
    "apply_throttle_patch",
    "ensure_raknet_transport",
    "ensure_shader_compatibility",
    "delete_roblox",
    "required_client_update",
]


def installed_version():
    try:
        with open(locations.APP_BUNDLE / "Contents" / "Info.plist", "rb") as file:
            version = plistlib.load(file).get("CFBundleShortVersionString")
    except Exception:  # missing or damaged: plistlib raises several kinds
        return None
    return version if isinstance(version, str) else None


def _validated_deployment(data):
    if not isinstance(data, dict):
        raise ValueError("Invalid Roblox deployment response")
    version, upload = data.get("version"), data.get("clientVersionUpload")
    if (
        not isinstance(version, str)
        or not re.fullmatch(r"\d{1,10}(?:\.\d{1,10}){3}", version)
        or not isinstance(upload, str)
        or not re.fullmatch(r"version-[0-9a-fA-F]{16,64}", upload)
    ):
        raise ValueError("Invalid Roblox deployment version or upload identifier")
    return version, upload


def version_is_newer(candidate, installed):
    try:
        return tuple(map(int, candidate.split("."))) > tuple(
            map(int, installed.split("."))
        )
    except (AttributeError, TypeError, ValueError):
        return candidate != installed


def latest_version(channel="", required_log=None):
    """Check the public release unless a required update names a channel.

    A previously reported rollout channel may be restricted to signed-in
    clients. It must never become the default for setup or routine checks.
    """
    if not isinstance(channel, str) or (
        channel and not re.fullmatch(r"[A-Za-z0-9_-]{1,64}", channel)
    ):
        raise RuntimeError("Invalid Roblox update channel")
    url = locations.VERSION_URL + ("/channel/" + channel if channel else "")
    try:
        with open_public(url, timeout=10) as response:
            data = json.loads(response.read(65536))
    except urllib.error.HTTPError as error:
        if channel and error.code in (401, 403):
            target = required_client_update(required_log)
            if (
                target
                and target.get("channel") == channel
                and target.get("upload")
                and version_is_newer(target["version"], installed_version())
            ):
                return target["version"], target["upload"]
            raise RuntimeError(
                f"Roblox restricts update checks for channel {channel}. Start Roblox and use its required-update dialog to install the version requested by the client."
            ) from error
        raise
    return _validated_deployment(data)


def update_roblox(upload, progress=None, expected_version=None):
    """Download the official macOS client and swap it in, keeping fast flags.
    The previous bundle is moved to backups/, and that path is returned (None
    when there was no client before). progress(fraction, text)."""
    if not isinstance(upload, str) or not re.fullmatch(
        r"version-[0-9a-fA-F]{16,64}", upload
    ):
        raise RuntimeError("Invalid Roblox client upload identifier")
    locations.DOWNLOADS.mkdir(parents=True, exist_ok=True)
    archive = locations.DOWNLOADS / f"{upload}-RobloxPlayer.zip"
    try:

        def fallback_progress():
            if progress:
                progress(0.0, _("Roblox returned 403; trying another download server"))

        with (
            open_public(
                locations.DOWNLOAD_URL.format(upload=upload),
                timeout=30,
                on_fallback=fallback_progress,
            ) as response,
            open(archive, "wb") as out,
        ):
            total = int(response.headers.get("Content-Length") or 0)
            done = 0
            while chunk := response.read(1 << 16):
                out.write(chunk)
                done += len(chunk)
                if progress and total:
                    progress(
                        done / total * 0.9,
                        _(
                            "Downloading {done} of {total} MB",
                            done=done >> 20,
                            total=total >> 20,
                        ),
                    )
            if total and done != total:
                raise OSError(
                    f"Incomplete Roblox download: received {done} of {total} bytes. The installed client was kept."
                )
    except BaseException:
        archive.unlink(missing_ok=True)  # no half-downloaded client left behind
        raise
    # Unpack next to the bundle: the swap below is a rename, which fails
    # across filesystems (downloads/ may be a link to another disk).
    unpack = Path(tempfile.mkdtemp(prefix=".unpack-", dir=locations.DATA_DIR))
    try:
        if not zipfile.is_zipfile(archive):
            raise RuntimeError(_("The download is not a zip archive"))
        if progress:
            progress(0.92, _("Unpacking"))
        # unzip keeps the executable bits, zipfile does not.
        subprocess.run(["unzip", "-q", str(archive), "-d", str(unpack)], check=True)
        new_bundle = unpack / "RobloxPlayer.app"
        if not new_bundle.is_dir():
            raise RuntimeError(_("The archive has no RobloxPlayer.app"))
        if expected_version is not None:
            with (new_bundle / "Contents" / "Info.plist").open("rb") as file:
                downloaded = plistlib.load(file).get("CFBundleShortVersionString")
            if downloaded != expected_version:
                raise RuntimeError(
                    f"Roblox download version mismatch: requested {expected_version}, received {downloaded}. The installed client was kept."
                )
        flags = fast_flags.load_bundle_flags()
        old_version = installed_version() or "unknown"
        locations.BACKUPS.mkdir(parents=True, exist_ok=True)
        backup = locations.BACKUPS / f"RobloxPlayer-{old_version}.app"
        if backup.exists():
            shutil.rmtree(backup)
        had_bundle = locations.APP_BUNDLE.exists()
        if had_bundle:
            locations.APP_BUNDLE.rename(backup)
        try:
            new_bundle.rename(locations.APP_BUNDLE)
        except OSError:
            if had_bundle:
                backup.rename(locations.APP_BUNDLE)  # never leave the user without a client
            raise
    finally:
        shutil.rmtree(unpack, ignore_errors=True)
        archive.unlink(missing_ok=True)
    if had_bundle:
        # Keep only the newest backup: each is a whole client of several
        # hundred MB. (Not on a first install: an older backup may then be
        # the only other client there is.)
        for old in locations.BACKUPS.glob("RobloxPlayer-*.app"):
            if old != backup:
                shutil.rmtree(old, ignore_errors=True)
    if flags:
        fast_flags.save_bundle_flags(flags)
    apply_throttle_patch()
    ensure_raknet_transport()
    ensure_shader_compatibility()
    if progress:
        progress(1.0, _("Done"))
    return backup if had_bundle else None


def throttle_patch_state():
    """Whether the startup render throttle is patched out of the installed
    client: 'patched', 'original' or 'unsupported' (no client, or a build
    whose code shape the patcher does not recognize)."""
    try:
        r = subprocess.run(
            [
                sys.executable,
                str(locations.PROJECT / "patch_startup_throttle.py"),
                str(locations.APP_BUNDLE),
                "--check",
            ],
            capture_output=True,
            text=True,
            timeout=120,
        )
    except Exception:
        return "unsupported"
    out = (r.stdout or "").strip()
    return out if out in ("patched", "original") else "unsupported"


def run_throttle_patcher(*flags):
    """Run the patcher and return its output; raises on a failure exit."""
    try:
        r = subprocess.run(
            [
                sys.executable,
                str(locations.PROJECT / "patch_startup_throttle.py"),
                str(locations.APP_BUNDLE),
                *flags,
            ],
            capture_output=True,
            text=True,
            timeout=120,
        )
    except Exception as e:
        raise RuntimeError(str(e)) from e
    if r.returncode != 0:
        raise RuntimeError(
            (r.stderr or r.stdout or "").strip() or f"exit {r.returncode}"
        )
    return (r.stdout or "").strip()


def remove_throttle_patch():
    """Restore the original client bytes; the inverse of the patch."""
    return run_throttle_patcher("--undo")


def apply_throttle_patch():
    """Re-apply the startup-throttle patch to the client binary (see
    patch_startup_throttle.py at the project root).

    The macOS client throttles the menu to ~3 FPS for its first 10 seconds
    under Darling (the normal release path needs a preRenderJob that is never
    created here). The patcher locates its site by pattern, so it survives
    client updates and skips itself when the code shape changes. Runs on
    installs, updates and every launch when auto_patch_throttle is on; a run
    costs ~0.3 s."""
    if not config.load_settings().get("auto_patch_throttle", True):
        return
    try:
        output = run_throttle_patcher()
    except Exception as e:
        logging.getLogger("quadra").warning("Startup throttle patch failed: %s", e)
        return
    if "(patched at" in output:
        logging.getLogger("quadra").info(output)
    elif "not found" in output:
        logging.getLogger("quadra").warning("Startup throttle patch: %s", output)


    # "already disabled" stays silent.


def ensure_raknet_transport():
    """Ensure FastFlags in ClientAppSettings.json and the client binary disable
    RbxTransport (QUIC) and enforce RakNet. Under Darling, RbxTransport fails socket
    connection, causing an ~11 s freeze before Roblox disconnects with Error 256."""
    try:
        flags = {**fast_flags.load_fast_flags(), **fast_flags.DARLING_FLAGS}
        if flags != fast_flags.load_bundle_flags():
            fast_flags.save_bundle_flags(flags)

        binary = locations.APP_BUNDLE / "Contents" / "MacOS" / "RobloxPlayer"
        if binary.is_file():
            from ..darling.transport_patches import apply_transport_patch

            status = apply_transport_patch(binary)
            if status.startswith("unsupported"):
                data = binary.read_bytes()
                status += (
                    f"; size={len(data)}; sha256={hashlib.sha256(data).hexdigest()}"
                )
                logging.getLogger("quadra").warning("RakNet compatibility: %s", status)
            elif status == "patched":
                logging.getLogger("quadra").info(
                    "Verified RakNet compatibility patches applied"
                )
            return status
        return "client not installed"
    except Exception as e:
        logging.getLogger("quadra").warning("Failed to ensure RakNet transport: %s", e)
        return f"compatibility preparation failed: {e}"


def ensure_shader_compatibility():
    """Repair the verified GLSL pack's HeightmapDebugPS parser failure."""
    pack = locations.APP_BUNDLE / "Contents" / "Resources" / "shaders" / "shaders_glsl3.pack"
    if not pack.is_file():
        return "shader pack not installed"
    try:
        from ..darling.shader_patches import apply_shader_patch

        status = apply_shader_patch(pack)
        if status.startswith("unsupported"):
            data = pack.read_bytes()
            status += f"; size={len(data)}; sha256={hashlib.sha256(data).hexdigest()}"
            logging.getLogger("quadra").warning("GLSL compatibility: %s", status)
        elif status == "patched":
            logging.getLogger("quadra").info(
                "Verified HeightmapDebugPS shader compatibility repair applied"
            )
        return status
    except Exception as e:
        logging.getLogger("quadra").warning(
            "Failed to ensure GLSL compatibility: %s", e
        )
        return f"compatibility preparation failed: {e}"


def delete_roblox():
    """Completely delete the local RobloxPlayer.app bundle and any mod backups."""
    if locations.APP_BUNDLE.exists():
        if locations.APP_BUNDLE.is_dir():
            shutil.rmtree(locations.APP_BUNDLE, ignore_errors=True)
        else:
            locations.APP_BUNDLE.unlink(missing_ok=True)
    mods_backup = locations.DATA_DIR / "mods_backup"
    if mods_backup.exists() and mods_backup.is_dir():
        shutil.rmtree(mods_backup, ignore_errors=True)


def required_client_update(log_path):
    """Validated deployment requested by this client, including gated channels.

    Do not forward logged URLs or authentication. Uploads are downloaded only
    from our fixed official CDN URL, and their bundle version is checked before
    replacing the installed client. A channel without a response is retained
    so recovery cannot silently check the unrelated production deployment.
    """
    tail = logs._log_tail(log_path, 262144)
    if (
        "flow end: app_closed_for_update" not in tail
        and "Found new version and the updater launched. Drain reporting and quit."
        not in tail
    ):
        return None
    channels = re.findall(
        r"\[FLog::UpdateController\] UpdateController: versionQueryUrl: "
        r"https://clientsettingscdn\.roblox\.com/v2/client-version/MacPlayer"
        r"(?:/channel/([A-Za-z0-9_-]{1,64}))?(?=\s|$)",
        tail,
    )
    target = {"channel": channels[-1]} if channels else {}
    decoder = json.JSONDecoder()
    for match in re.finditer(r"\[FLog::UpdateController\] version response:\s*", tail):
        try:
            data, _end = decoder.raw_decode(tail[match.end() : match.end() + 4096])
            version, upload = _validated_deployment(data)
            target.update(version=version, upload=upload)
        except (ValueError, TypeError):
            continue
    # A second launch can choose FORCE using a valid UpdateController cache
    # without logging another network response. Never use another channel's
    # cache (the production/zbeta mismatch is what caused this update loop).
    if (
        "upload" not in target
        and "channel" in target
        and "[FLog::UpdateController] Cache valid (" in tail
        and "[FLog::UpdateController] Cache channel mismatch" not in tail
    ):
        for match in re.finditer(
            r'\[FLog::UpdateController\] channel: "([A-Za-z0-9_-]{0,64})", '
            r"timestamp: \d+, json:\s*",
            tail,
        ):
            if match.group(1) != target["channel"]:
                continue
            try:
                data, _end = decoder.raw_decode(tail[match.end() : match.end() + 4096])
                version, upload = _validated_deployment(data)
                target.update(version=version, upload=upload)
            except (ValueError, TypeError):
                continue
    return target or None
