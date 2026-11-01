"""Running Roblox through Rosemary, the other macOS translation layer.

Rosemary (https://github.com/microlift/rosemary) runs macOS programs on Linux
without Darling. Roblox starts as

    ROSEMARY_PREFIX=<prefix> rosemary run [--native|--emulated] <Roblox.app> [-protocolString <link>]

from the prefix's root/ folder, which the client sees as /. A join link is
passed the way Rosemary's tools/install-url-handler.sh passes one.

Adapted from nutmeg (https://github.com/frolleks/nutmeg, GPL-3.0-or-later,
Copyright frolleks), nutmeg/rosemary.py and nutmeg/butter.py. Not covered
here: Butter bottles (their prefix settings), nutmeg's wrapper command."""

import logging
import os
import shutil
import signal
import subprocess
import threading
import time
import tomllib
from pathlib import Path

from . import rosemary_client
from .paths import user_directory

log = logging.getLogger("quadra.rosemary")

# Variables the settings below control: removed from the inherited
# environment so an exported value cannot silently override the settings.
MANAGED_ENV = ("ROSEMARY_PREFIX", "ROSEMARY_SWAP_INTERVAL", "ROSEMARY_SERVER", "SDL_VIDEO_DRIVER",
               "SDL_VIDEODRIVER")
ENGINES = ("auto", "native", "emulated")
SWAP_INTERVALS = ("", "0", "1", "-1")
VIDEO_DRIVERS = ("", "wayland", "x11")
WEBSITE = "https://github.com/microlift/butter"


class RosemaryError(Exception):
    """Worded for the person using Quadra; `settings` says a setting is the fix."""

    def __init__(self, message, *, settings=False):
        super().__init__(message)
        self.settings = settings


# -- Butter's runners

def butter_data_dirs():
    base = Path(os.environ.get("XDG_DATA_HOME") or Path.home() / ".local" / "share")
    return [base / "butter", Path.home() / ".var" / "app" / "io.github.microlift.butter" / "data" / "butter"]


def _version(tag):
    import re
    if not tag:
        return ()
    return tuple(int(part) for part in re.findall(r"\d+", str(tag).split("-", 1)[0]))


def runners():
    """[(name, executable)] of Butter's installed Rosemary runners, newest first."""
    found = []
    for data in butter_data_dirs():
        try:
            directories = sorted(d for d in (data / "runners").iterdir() if d.is_dir() and not d.name.startswith("."))
        except OSError:
            continue
        for directory in directories:
            try:
                info = tomllib.loads((directory / "runner.toml").read_text("utf-8"))
            except (OSError, ValueError):
                continue
            name = info.get("executable")
            if not isinstance(name, str) or not name:
                continue
            program = directory / name
            if program.is_file():
                tag = info.get("tag")
                found.append((str(info.get("name") or directory.name), program,
                              info.get("kind") == "custom", _version(tag)))
    found.sort(key=lambda r: (r[2], tuple(-n for n in r[3]), r[0]))
    return [(name, program) for name, program, _custom, _tag in found]


def quadra_runners():
    """[(name, executable)] of Rosemary builds unpacked under Quadra's own
    runner folder (rosemary-runner/<version>/squashfs-root/bin/rosemary, from a
    release AppImage extracted with --appimage-extract, or <version>/bin/rosemary)."""
    root = user_directory("DATA") / "rosemary-runner"
    found = []
    try:
        versions = sorted((d for d in root.iterdir() if d.is_dir()), reverse=True)
    except OSError:
        return []
    for directory in versions:
        for relative in ("squashfs-root/bin/rosemary", "bin/rosemary"):
            program = directory / relative
            if program.is_file() and os.access(program, os.X_OK):
                found.append((f"Rosemary {directory.name}", program))
                break
    return found


def find_rosemary(settings):
    """The Rosemary program: the one chosen in settings, else Quadra's own
    newest runner, else Butter's newest, else the one on PATH."""
    chosen = (settings.get("rosemary_program") or "").strip()
    if chosen:
        path = Path(chosen).expanduser()
        if not path.is_file() or not os.access(path, os.X_OK):
            raise RosemaryError(f"{path} is not an executable file. Choose another Rosemary program in "
                                "Settings → Rosemary.", settings=True)
        return path
    available = quadra_runners() or runners()
    if available:
        return available[0][1]
    found = shutil.which("rosemary")
    if found:
        return Path(found)
    raise RosemaryError("Rosemary is not installed. Press Install in Settings → Rosemary, or install a "
                        f"runner with Butter ({WEBSITE}), or choose a Rosemary program there.", settings=True)


# -- prefix

def default_prefix():
    return rosemary_client.data_dir() / "prefix"


def prefix_path(settings):
    chosen = (settings.get("rosemary_prefix") or "").strip()
    return Path(chosen).expanduser() if chosen else default_prefix()


def prefix_created(prefix):
    """`rosemary prefix create` writes this file last."""
    return (prefix / ".rosemary" / "state.yaml").is_file()


def create_prefix(rosemary, prefix, output=lambda line: None, timeout=300):
    """`rosemary prefix create --no-base`: the directories only. Programs run
    without the Darwin base, because Rosemary links them against its own
    system libraries (nutmeg's README)."""
    prefix.mkdir(parents=True, exist_ok=True)
    result = subprocess.run([str(rosemary), "prefix", "create", str(prefix), "--no-base"],
                            stdin=subprocess.DEVNULL, capture_output=True, text=True, timeout=timeout)
    for line in (result.stdout + result.stderr).splitlines():
        output(line)
    if result.returncode != 0:
        tail = "\n".join((result.stdout + result.stderr).splitlines()[-12:])
        raise RosemaryError(f"Rosemary could not create the prefix (exit status {result.returncode}):\n{tail}")


# -- environment and command line

def run_env(settings, base=None):
    env = dict(os.environ if base is None else base)
    for name in MANAGED_ENV:
        env.pop(name, None)
    env["ROSEMARY_PREFIX"] = str(prefix_path(settings))
    interval = settings.get("rosemary_swap_interval", "")
    if interval in SWAP_INTERVALS and interval:
        env["ROSEMARY_SWAP_INTERVAL"] = interval
    if settings.get("rosemary_process_server", True) is False:
        env["ROSEMARY_SERVER"] = "off"
    driver = settings.get("rosemary_video_driver", "")
    if driver in VIDEO_DRIVERS and driver:
        env["SDL_VIDEO_DRIVER"] = driver
    if settings.get("mangohud", False):
        # The `mangohud` wrapper preloads host libraries into Rosemary, which
        # carries its own glibc: the first run crashed with "free(): invalid
        # size". The Vulkan layer is switched on by this variable instead.
        env["MANGOHUD"] = "1"
    if settings.get("rosemary_force_opengl", False):
        # With no Vulkan driver Rosemary's Metal cannot start and Roblox uses
        # its OpenGL renderer.
        env["VK_DRIVER_FILES"] = "/dev/null"
    return env


def run_argv(rosemary, settings, app, url):
    """The command that starts Roblox. `app` is the Roblox.app folder."""
    # MangoHud is not wrapped around Rosemary (see run_env).
    argv = [str(rosemary), "run"]
    mode = settings.get("rosemary_engine", "auto")
    if mode == "native":
        argv.append("--native")
    elif mode == "emulated":
        argv.append("--emulated")
    argv.append(str(app))
    if url is not None:
        argv += ["-protocolString", url]
    return argv


def choose_arch(settings):
    """Which Roblox build to install: this machine's, unless emulated x86_64."""
    host = rosemary_client.host_arch()
    if host is None:
        raise RosemaryError("Roblox for macOS has no build for this machine.")
    mode = settings.get("rosemary_engine", "auto")
    if mode == "emulated":
        if host != "x86_64":
            return "x86_64"  # Rosemary can only emulate x86_64
    return host


def player_log_dirs(prefix):
    """Folders where the client writes its logs (guest home: Library/Logs/Roblox)."""
    found = sorted((prefix / "root" / "Users").glob("*/Library/Logs/Roblox"))
    found.append(Path.home() / "Library" / "Logs" / "Roblox")
    return found


def newest_player_log(prefix, since=0.0):
    """The newest Roblox player log written after `since`, or None."""
    best, best_time = None, since
    for directory in player_log_dirs(prefix):
        try:
            for path in directory.glob("*.log"):
                modified = path.stat().st_mtime
                if modified >= best_time:
                    best, best_time = path, modified
        except OSError:
            continue
    return best


def installed():
    """(Build, bundle) of the newest intact installed build, or None."""
    builds = rosemary_client.installed_builds()
    return builds[0] if builds else None


def installed_version():
    found = installed()
    return (found[0].version or found[0].guid) if found else None


def ensure_latest(settings, progress, cancel=None):
    """Install the current build when it is not installed. progress(fraction,
    text). Returns the bundle path. Offline, the installed build is used."""
    cancel = cancel or threading.Event()
    arch = choose_arch(settings)
    have = installed()
    try:
        latest = rosemary_client.fetch_latest(arch)
    except rosemary_client.ClientError:
        if have:
            return have[1]
        raise
    if have and have[0].key == latest.key:
        return have[1]
    progress(0.0, f"Downloading Roblox {latest.label}")
    zip_path = rosemary_client.download(
        latest, lambda done, total: progress((done / total * 0.8) if total else 0.0,
                                             f"Downloading Roblox {latest.label}"), cancel)
    progress(0.8, "Installing Roblox")
    try:
        rosemary_client.install(zip_path, latest, lambda done, total: progress(0.8 + 0.2 * done / total,
                                                                               "Installing Roblox"), cancel)
    finally:
        zip_path.unlink(missing_ok=True)
    rosemary_client.remove_old(latest.key)
    progress(1.0, "Done")
    return latest.bundle()


def running():
    """Whether a Rosemary program (a game or the process server) is running."""
    try:
        entries = os.listdir("/proc")
    except OSError:
        return False
    for entry in entries:
        if not entry.isdigit():
            continue
        try:
            with open(f"/proc/{entry}/cmdline", "rb") as handle:
                argv = handle.read().split(b"\0")
        except OSError:
            continue
        if argv and os.path.basename(argv[0].decode("utf-8", "replace")) == "rosemary" and len(argv) > 1 \
                and argv[1] in (b"run", b"server"):
            return True
    return False


# -- the game

class RosemarySession:
    """One run of the client through Rosemary; the same surface as
    core.RobloxSession that the launcher window uses. poll() returns None
    while it runs, then the exit status."""

    def __init__(self, settings, launch_uri=None, logs_dir=None):
        self.settings = settings
        self.launch_uri = launch_uri       # opaque: given to the client as it is
        self.log_path = None
        self.logs_dir = Path(logs_dir) if logs_dir else None
        self.process = None
        self.started_at = 0.0
        self.rosemary = None
        self.prefix = None
        self._finished = False
        # The surface of core.RobloxSession the window touches.
        self.shader_preload_progress = None
        self.preload_stage_progress = None
        self.shader_preload_cancel = threading.Event()
        self.web_socket = None
        self.web_user_agent = None
        self.metal_fallback_reason = None
        self.notes = []
        from .transport_monitor import TransportMonitor
        self.transport_monitor = TransportMonitor()

    def game_log(self):
        """The client's own log (FLog lines: joins, teleports), once it exists."""
        return newest_player_log(self.prefix, self.started_at - 2) if self.prefix else None

    def start(self):
        found = installed()
        if found is None:
            raise RosemaryError("Roblox is not installed for Rosemary yet.")
        _build, bundle = found
        self.rosemary = find_rosemary(self.settings)
        self.prefix = prefix_path(self.settings)
        if not prefix_created(self.prefix):
            create_prefix(self.rosemary, self.prefix)
        from . import core
        flags = core.load_fast_flags()
        if flags:
            rosemary_client.write_fast_flags(bundle, flags)
        logs = self.logs_dir or core.LOGS
        logs.mkdir(parents=True, exist_ok=True)
        self.log_path = logs / time.strftime("launch-%Y%m%d-%H%M%S-rosemary.log")
        argv = run_argv(self.rosemary, self.settings, bundle, self.launch_uri)
        env = run_env(self.settings)
        root = self.prefix / "root"
        root.mkdir(parents=True, exist_ok=True)
        with open(self.log_path, "wb") as log_file:
            log_file.write(f"[Quadra Lifecycle] runtime=rosemary rosemary={self.rosemary} "
                           f"prefix={self.prefix} app={bundle}\n".encode())
            log_file.flush()
            self.started_at = time.time()
            self.process = subprocess.Popen(argv, cwd=root, env=env, stdin=subprocess.DEVNULL,
                                            stdout=log_file, stderr=subprocess.STDOUT,
                                            start_new_session=True)

    def poll(self):
        if self.process is None:
            return -1
        status = self.process.poll()
        if status is None:
            return None
        self.finish("exited", frontend_status=status)
        return status if status >= 0 else 128 - status  # killed by a signal

    def finish(self, reason="explicit_cleanup", frontend_status=None):
        """End the game and its helpers (idempotent)."""
        if self._finished:
            return
        self._finished = True
        self.stop_game()
        if self.settings.get("rosemary_process_server", True) and self.rosemary and self.prefix:
            try:
                subprocess.run([str(self.rosemary), "server", "--stop"], env=run_env(self.settings),
                               stdin=subprocess.DEVNULL, capture_output=True, timeout=15)
            except (OSError, subprocess.SubprocessError):
                pass

    def stop_game(self):
        """Ask Rosemary and its children to exit, then insist."""
        process = self.process
        if process is None or process.poll() is not None:
            return
        for sig, wait in ((signal.SIGTERM, 4.0), (signal.SIGKILL, 2.0)):
            try:
                os.killpg(process.pid, sig)
            except (ProcessLookupError, PermissionError):
                return
            end = time.monotonic() + wait
            while time.monotonic() < end:
                if process.poll() is not None:
                    return
                time.sleep(0.1)
