"""Where Quadra keeps things: the project, data, cache and config folders, the Roblox
bundle, the selected Darling runtime and its prefix, and the Roblox download URLs."""

import os
from pathlib import Path

from ..darling.runtime import resolve_runtime
from ..paths import migrate_directories, user_directory

__all__ = [
    "APP_BUNDLE",
    "BACKUPS",
    "BUILD_DIR",
    "BUILD_SCRIPT",
    "CACHE_DIR",
    "CONFIG_DIR",
    "DARLING_EXECUTABLE",
    "DARLING_HOME",
    "DARLING_PREFIX",
    "DARLING_RUNTIME",
    "DARLING_SYSROOT",
    "DATA_DIR",
    "DOWNLOADS",
    "DOWNLOAD_URL",
    "FAST_FLAGS",
    "FRAMEWORKS",
    "FRAMEWORKS_BUILD",
    "ICONS",
    "LAST_ERROR",
    "LOGS",
    "METAL_BUILD_SCRIPT",
    "METAL_DIR",
    "METAL_FRAMEWORK",
    "METAL_HOST_LIBRARY",
    "NATIVE_BUILD",
    "NATIVE_LIBS",
    "NOROOT_LIB",
    "PREBUILT_SHIM",
    "PROJECT",
    "QUIT_SENTINEL",
    "SESSION_DIR",
    "SESSION_FILES",
    "SETTINGS_FILE",
    "SHIM",
    "SHIM_DIR",
    "VERSION_URL",
    "WEB_DATA_DIR",
]

PROJECT = Path(__file__).resolve().parents[3]
# Everything the launcher writes: the project folder for a git checkout, the
# user's data folder when the sources are installed read-only (a package).
DATA_DIR = PROJECT if os.access(PROJECT, os.W_OK) else user_directory("DATA")
APP_BUNDLE = DATA_DIR / "RobloxPlayer.app"
BUILD_DIR = DATA_DIR / "build"
# A package may ship the shim built already (the Flatpak has no compiler).
PREBUILT_SHIM = os.environ.get("QUADRA_PREBUILT_SHIM")
SHIM_DIR = Path(PREBUILT_SHIM) if PREBUILT_SHIM else BUILD_DIR
SHIM = SHIM_DIR / "libQuadraShims.dylib"
# Frameworks RobloxPlayer links that Darling lacks; stubs from darling/frameworks/.
FRAMEWORKS = ["CoreML", "CoreHaptics", "DeviceCheck"]
FRAMEWORKS_BUILD = SHIM_DIR / "frameworks"
# Quadra M2VK (renderer "metal"): Metal.framework and the host library,
# built by darling/metal-vulkan/build.sh the first time the renderer is used.
METAL_DIR = SHIM_DIR / "metal"
METAL_HOST_LIBRARY = METAL_DIR / "libquadra_metal_host.so"
METAL_FRAMEWORK = METAL_DIR / "Metal.framework" / "Versions" / "A" / "Metal"
METAL_BUILD_SCRIPT = PROJECT / "darling" / "metal-vulkan" / "build.sh"
# Keep the selected client, build root and guest prefix together even when
# regular Darling and CiderDarling are both installed.
DARLING_RUNTIME = resolve_runtime()
DARLING_EXECUTABLE = DARLING_RUNTIME["executable"]
DARLING_SYSROOT = Path(DARLING_RUNTIME["sysroot"])
DARLING_PREFIX = Path(DARLING_RUNTIME["prefix"])
# Rootless Darling (for sandboxes such as Flatpak, see flatpak/darling-noroot.c):
# this library is preloaded into `darling` only, never into the launcher.
NOROOT_LIB = os.environ.get("QUADRA_NOROOT_LIB")
# Darling's bridges to host libraries that Roblox never uses, relative to the
# macOS root: when the host lacks the library, they are patched to load nothing.
NATIVE_LIBS = [
    f"usr/lib/native/{name}.dylib"
    for name in (
        "libavcodec",
        "libavformat",
        "libavutil",
        "libswresample",
        "libjpeg",
        "libfuse",
    )
]
NATIVE_LIBS.append(
    "System/Library/Frameworks/OpenGL.framework/Versions/A/Libraries/libGLU.dylib"
)
NATIVE_BUILD = BUILD_DIR / "native"
BUILD_SCRIPT = PROJECT / "build_debug_shim.sh"
LOGS = DATA_DIR / "logs"
BACKUPS = DATA_DIR / "backups"
DOWNLOADS = DATA_DIR / "downloads"
ICONS = PROJECT / "branding" / "icons"
FAST_FLAGS = (
    APP_BUNDLE / "Contents" / "MacOS" / "ClientSettings" / "ClientAppSettings.json"
)

CONFIG_DIR = user_directory("CONFIG")
CACHE_DIR = user_directory("CACHE")
# Written by the shim when Roblox starts terminating; see RobloxSession.poll.
QUIT_SENTINEL = CACHE_DIR / "game-closing"
# The last error shown in a dialog, for reports; cleared after a success.
LAST_ERROR = CACHE_DIR / "last-error.txt"
SETTINGS_FILE = CONFIG_DIR / "settings.json"
# Cookies and site data of Roblox's embedded web pages (web.py): sign-in state.
WEB_DATA_DIR = CONFIG_DIR / "web"

DARLING_HOME = DARLING_PREFIX / "Users" / os.environ.get("USER", "user")
SESSION_DIR = migrate_directories(
    DARLING_HOME / "Library" / "QuadraHQ",
    DARLING_HOME / "Library" / "CiderHQ",
    DARLING_HOME / "Library" / "MacOBlox",
)
SESSION_FILES = [
    SESSION_DIR / "Cookies.plist",
    SESSION_DIR / "Keychain",
]

VERSION_URL = "https://clientsettingscdn.roblox.com/v2/client-version/MacPlayer"
DOWNLOAD_URL = "https://setup.rbxcdn.com/mac/{upload}-RobloxPlayer.zip"
