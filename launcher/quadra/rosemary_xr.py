"""VR for Roblox under Rosemary (the other half of Quadra's OpenXR support).

Quadra's VR device (vr_device.c, vr_xr.c) is injected into RobloxPlayer as
libQuadraVR.dylib. That needs two things Rosemary 0.5.0 does not do: insert a
library into Roblox's hardened binary (ROSEMARY_DYLD_INSERT_FORCE) and let the
library call the host's OpenXR loader (the quadra_host_* exports). Both are in
Quadra's fork of Rosemary (QuadraHQ/rosemary-fork, branch quadra-vr; see its
docs/quadra-vr.md). With any other Rosemary VR stays off and the launch log
says why. Works only with the native x86-64 backend and Roblox's OpenGL
renderer, which this forces for a VR session."""

import hashlib
import os
import subprocess
import threading
from pathlib import Path

from . import vr
from .i18n import _

FORCE_VARIABLE = b"ROSEMARY_DYLD_INSERT_FORCE"
SOURCES = ("vr_device.c", "vr_xr.c", "vr_xr.h", "vr_openxr.h", "rosemary_vr/rosemary_glue.c")
# Roblox's OpenGL renderer: the VR device attaches to DeviceGL, not Metal.
OPENGL_FLAGS = {"FFlagDebugGraphicsPreferOpenGL": "True", "FFlagDebugGraphicsDisableMetal": "True"}

_support_cache = {}
_build_lock = threading.Lock()


class VrUnavailable(Exception):
    """VR cannot start for a reason worded for the user."""


def _binaries(program):
    """The files that hold Rosemary's code: the program itself, and for a release bundle
    (an unpacked AppImage) the real runtime in libexec/, which bin/rosemary only launches."""
    program = Path(program)
    found = [program]
    try:
        found.append(program.resolve().parent.parent / "libexec" / "rosemary")
    except OSError:
        pass
    return [path for path in found if path.is_file()]


def _has_override(path):
    try:
        with open(path, "rb") as handle:
            tail = b""
            while chunk := handle.read(8 << 20):
                if FORCE_VARIABLE in tail + chunk:
                    return True
                tail = chunk[-len(FORCE_VARIABLE):]
    except OSError:
        pass
    return False


def supports_vr(program):
    """Whether this Rosemary has the force-insert override (Quadra's build)."""
    files = _binaries(program)
    keys = []
    for path in files:
        try:
            status = os.stat(path)
        except OSError:
            return False
        keys.append((str(path), status.st_mtime_ns, status.st_size))
    key = tuple(keys)
    if key not in _support_cache:
        _support_cache[key] = any(_has_override(path) for path in files)
    return bool(files) and _support_cache[key]


def build_directory(core):
    return core.CACHE_DIR / "rosemary-vr"


def dylib_path(core):
    return build_directory(core) / "libQuadraVR.dylib"


def _stale(core):
    target = dylib_path(core)
    if not target.is_file():
        return True
    built = target.stat().st_mtime
    return any((core.PROJECT / name).stat().st_mtime > built for name in SOURCES if (core.PROJECT / name).exists())


def ensure_dylib(core, runner=subprocess.run):
    """Build libQuadraVR.dylib when it is missing or older than its sources."""
    with _build_lock:
        if not _stale(core):
            return dylib_path(core)
        script = core.PROJECT / "rosemary_vr" / "build_rosemary_vr.sh"
        if not script.is_file():
            raise VrUnavailable(_("This Quadra install has no VR build script (rosemary_vr/)."))
        build_directory(core).mkdir(parents=True, exist_ok=True)
        env = dict(os.environ, QUADRA_BUILD_DIR=str(build_directory(core)))
        try:
            result = runner(["bash", str(script)], env=env, stdin=subprocess.DEVNULL, capture_output=True,
                            text=True, timeout=300)
        except (OSError, subprocess.SubprocessError) as error:
            raise VrUnavailable(_("Could not build the VR library: {error}").format(error=error)) from error
        if result.returncode != 0 or not dylib_path(core).is_file():
            tail = (result.stderr or result.stdout or "").strip().splitlines()[-4:]
            raise VrUnavailable(_("Could not build the VR library (needs clang and Darling's runtime files):")
                                + "\n" + "\n".join(tail))
        return dylib_path(core)


def mesa_egl_vendor(search=("/usr/share/glvnd/egl_vendor.d", "/etc/glvnd/egl_vendor.d")):
    """Mesa's EGL vendor file (zink is part of Mesa), or None."""
    for directory in search:
        try:
            for name in sorted(os.listdir(directory)):
                if "mesa" in name and name.endswith(".json"):
                    return os.path.join(directory, name)
        except OSError:
            continue
    return None


class VrSession:
    """What a VR start adds to Rosemary's environment and the client's flags."""

    def __init__(self, dylib, variables, log, zink_vendor=None):
        self.dylib = dylib
        self.variables = variables      # QUADRA_* lines for the guest
        self.flags = dict(OPENGL_FLAGS)
        self.log = log
        self.zink_vendor = zink_vendor  # Mesa's EGL vendor file: run OpenGL on Vulkan (zink)

    def apply(self, env):
        """Add the injection and the guest variables to a host environment dict."""
        libraries = [str(self.dylib)] + [lib for lib in env.get("ROSEMARY_DYLD_INSERT_LIBRARIES", "").split(":")
                                         if lib and lib != str(self.dylib)]
        env["ROSEMARY_DYLD_INSERT_LIBRARIES"] = ":".join(libraries)
        env["ROSEMARY_DYLD_INSERT_FORCE"] = "1"
        lines = [line for line in env.get("ROSEMARY_ENVIRON", "").split("\n") if line]
        lines += [line for line in self.variables if line not in lines]
        env["ROSEMARY_ENVIRON"] = "\n".join(lines)
        env["SDL_VIDEO_FORCE_EGL"] = "1"   # the OpenXR session binds to an EGL context
        if self.zink_vendor:
            # Roblox's VR needs its OpenGL renderer; zink runs that on Vulkan.
            env["MESA_LOADER_DRIVER_OVERRIDE"] = "zink"
            env["__EGL_VENDOR_LIBRARY_FILENAMES"] = self.zink_vendor
        return env


def prepare(settings, program, core):
    """A VrSession when the settings ask for VR and everything is in place; None
    when VR is off. Raises VrUnavailable when it is asked for but cannot start."""
    if not settings.get("vr", False):
        return None
    if not supports_vr(program):
        raise VrUnavailable(_("This Rosemary cannot load VR. Quadra's own build is needed "
                              "(QuadraHQ/rosemary-fork); choose it under Settings → Rosemary."))
    variables, log = vr.shim_variables({**settings, "renderer": "opengl"})
    if not variables:
        raise VrUnavailable(log.removeprefix("VR: "))
    vendor = None
    if settings.get("rosemary_vr_zink", False):
        vendor = mesa_egl_vendor()
        if vendor is None:
            raise VrUnavailable(_("OpenGL on Vulkan (zink) needs Mesa's EGL; its vendor file was not found."))
        log += "; OpenGL through zink (Vulkan)"
    return VrSession(ensure_dylib(core), variables, log + " (Rosemary)", vendor)
