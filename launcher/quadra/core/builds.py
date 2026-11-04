"""Building the shim and the Quadra M2VK Metal framework, and the programs that needs."""

import hashlib
import os
import shutil
import subprocess

from .. import __version__
from ..i18n import _
from . import files, locations

__all__ = [
    "SHIM_STAMP",
    "_metal_sources_hash",
    "_shim_sources_hash",
    "build_metal",
    "build_shim",
    "metal_built",
    "missing_tools",
    "shim_built",
]

SHIM_STAMP = locations.BUILD_DIR / "sources.sha256"


def _shim_sources_hash():
    """Hash of everything the shim build uses, and of the launcher version."""
    digest = hashlib.sha256(__version__.encode())
    digest.update(str(locations.DARLING_SYSROOT).encode())
    digest.update(
        str(locations.DARLING_RUNTIME["metadata"].get("source_revision", "")).encode()
    )
    for path in sorted(
        [
            *(locations.PROJECT / "darling" / "shim").glob("*"),
            *(locations.PROJECT / "vr").glob("*"),
            *(locations.PROJECT / "darling" / "frameworks").glob("*"),
            locations.BUILD_SCRIPT,
        ]
    ):
        try:
            digest.update(path.name.encode() + b"\0" + path.read_bytes())
        except OSError:
            pass
    return digest.hexdigest()


def build_shim():
    if locations.PREBUILT_SHIM:
        return True, _("The shim comes built with this package")
    stamp = _shim_sources_hash()
    result = subprocess.run(
        [str(locations.BUILD_SCRIPT)],
        capture_output=True,
        text=True,
        env=dict(
            os.environ,
            QUADRA_BUILD_DIR=str(locations.BUILD_DIR),
            DARLING_SYSROOT=str(locations.DARLING_SYSROOT),
        ),
    )
    if result.returncode == 0:
        try:
            files._write_atomically(SHIM_STAMP, stamp + "\n")
        except OSError:
            pass
    return result.returncode == 0, (result.stdout + result.stderr).strip()


def shim_built():
    """Built, and from the current sources: after a launcher update (git
    pull, a package upgrade) the next start rebuilds the shim."""
    if not (
        locations.SHIM.exists()
        and all(
            (locations.FRAMEWORKS_BUILD / f"{name}.framework" / name).exists()
            for name in locations.FRAMEWORKS
        )
    ):
        return False
    if locations.PREBUILT_SHIM:
        return True
    try:
        return SHIM_STAMP.read_text().strip() == _shim_sources_hash()
    except OSError:
        return False


def _metal_sources_hash():
    digest = hashlib.sha256(__version__.encode())
    digest.update(str(locations.DARLING_SYSROOT).encode())
    root = locations.PROJECT / "darling" / "metal-vulkan"
    sources = [
        root / "quadra_metal.h",
        locations.METAL_BUILD_SCRIPT,
        *(root / "framework").glob("*"),
        root / "host" / "Cargo.toml",
        root / "host" / "Cargo.lock",
        *(root / "host" / "src").rglob("*.rs"),
    ]
    # Cargo.toml and Cargo.lock pin the immutable Quadra M2VK dependency.
    for path in sorted(sources):
        try:
            digest.update(
                str(path.relative_to(locations.PROJECT)).encode()
                + b"\0"
                + path.read_bytes()
            )
        except OSError:
            pass
    return digest.hexdigest()


def metal_built():
    if not (
        locations.METAL_HOST_LIBRARY.exists() and locations.METAL_FRAMEWORK.exists()
    ):
        return False
    if locations.PREBUILT_SHIM:
        return True
    try:
        return (
            locations.METAL_DIR / "sources.sha256"
        ).read_text().strip() == _metal_sources_hash()
    except OSError:
        return False


def build_metal():
    """darling/metal-vulkan/build.sh: the framework needs clang and lld, the host library
    Rust 1.98 or newer (its first build downloads its crates)."""
    if locations.PREBUILT_SHIM:
        return metal_built(), _("Quadra M2VK is not part of this package")
    if not shutil.which("cargo"):
        return False, _(
            "Quadra M2VK needs Rust (cargo) 1.98 or newer to build. Install Rust or select another renderer."
        )
    stamp = _metal_sources_hash()
    result = subprocess.run(
        [str(locations.METAL_BUILD_SCRIPT)],
        capture_output=True,
        text=True,
        env=dict(
            os.environ,
            QUADRA_BUILD_DIR=str(locations.BUILD_DIR),
            DARLING_SYSROOT=str(locations.DARLING_SYSROOT),
        ),
    )
    if result.returncode == 0:
        try:
            files._write_atomically(
                locations.METAL_DIR / "sources.sha256", stamp + "\n"
            )
        except OSError:
            pass
    return result.returncode == 0, (result.stdout + result.stderr).strip()[-4000:]


def missing_tools():
    """Programs the launcher needs that are not installed."""
    needed = {locations.DARLING_EXECUTABLE: "darling", "unzip": "unzip"}
    if not locations.PREBUILT_SHIM:
        needed.update({"clang": "clang", "ld.lld": "lld"})
    missing = [
        package for program, package in needed.items() if not shutil.which(program)
    ]
    if not locations.DARLING_SYSROOT.is_dir() and "darling" not in missing:
        missing.append(f"darling ({locations.DARLING_SYSROOT})")
    return missing
