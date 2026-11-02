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
import re
import os
import shutil
import signal
import subprocess
import threading
import time
import tomllib
from pathlib import Path
from typing import NamedTuple

from . import client as rosemary_client
from ..paths import user_directory

log = logging.getLogger("quadra.rosemary")

# Variables the settings below control: removed from the inherited
# environment so an exported value cannot silently override the settings.
MANAGED_ENV = ("ROSEMARY_PREFIX", "ROSEMARY_SWAP_INTERVAL", "ROSEMARY_SERVER", "SDL_VIDEO_DRIVER",
               "SDL_VIDEODRIVER", "ROSEMARY_METAL_PRESENTATION_MODE", "ROSEMARY_SCREEN_REFRESH_RATE",
               "ROSEMARY_DECORATIONS", "ROSEMARY_MENU", "ROSEMARY_CONFINE_POINTER",
               "ROSEMARY_CONTROL_EDIT_KEYS", "ROSEMARY_COMMAND_KEY", "ROSEMARY_PLAIN_CTRL_CLICK")
ENGINES = ("auto", "native", "emulated")
SWAP_INTERVALS = ("", "0", "1", "-1")
VIDEO_DRIVERS = ("", "wayland", "x11")
PRESENTATION_MODES = ("", "immediate", "fifo", "fifo_relaxed", "mailbox")
DECORATIONS = ("", "client", "server")
MENUS = ("", "titlebar", "strip", "none")
COMMAND_KEYS = ("", "super", "ctrl", "alt")
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


def quadra_runners(source=None):
    """[(name, executable)] of Rosemary builds unpacked under Quadra's own runner folder
    (rosemary-runner/<version>/squashfs-root/bin/rosemary, from a release AppImage extracted
    with --appimage-extract, or <version>/bin/rosemary), newest first. With `source`
    ("quadra" or "upstream") only that kind: Quadra's build has "quadra" in its folder name."""
    from . import install as rosemary_install
    root = user_directory("DATA") / "rosemary-runner"
    try:
        versions = sorted((d for d in root.iterdir() if d.is_dir() and not d.name.startswith(".")),
                          key=lambda d: tuple(int(n) for n in re.findall(r"\d+", d.name)), reverse=True)
    except OSError:
        return []
    found = []
    for directory in versions:
        if source and rosemary_install.runner_kind(directory.name) != source:
            continue
        for relative in ("squashfs-root/bin/rosemary", "bin/rosemary"):
            program = directory / relative
            if program.is_file() and os.access(program, os.X_OK):
                found.append((f"Rosemary {directory.name}", program))
                break
    return found


def describe(program):
    """A short name for a Rosemary program: which build and version, for the settings page."""
    from . import install as rosemary_install
    parts = Path(program).parts
    if "rosemary-runner" in parts:
        folder = parts[parts.index("rosemary-runner") + 1]
        return f"{rosemary_install.SOURCE_NAMES[rosemary_install.runner_kind(folder)]} {folder}"
    return str(program)


class Runner(NamedTuple):
    program: str
    label: str
    added_by_hand: bool


def _home_relative(program):
    try:
        return f"~/{program.relative_to(Path.home())}"
    except ValueError:
        return str(program)


def runner_list(settings):
    listed = {}

    def offer(program, label, added_by_hand=False):
        program = Path(program).expanduser()
        listed.setdefault(str(program), Runner(str(program), label or _home_relative(program), added_by_hand))

    for _name, program in quadra_runners():
        offer(program, describe(program))
    for name, program in runners():
        offer(program, f"{name} (Butter)")
    on_path = shutil.which("rosemary")
    if on_path:
        offer(on_path, "rosemary on PATH")
    by_hand = [p for p in settings.get("rosemary_runners") or [] if isinstance(p, str)]
    for program in [*by_hand, settings.get("rosemary_program") or ""]:
        if program.strip():
            offer(program.strip(), None, added_by_hand=True)
    return list(listed.values())


def user_runner_list(settings):
    quadra_programs = {str(program) for _name, program in quadra_runners()}
    return [runner for runner in runner_list(settings) if runner.program not in quadra_programs]


def is_runnable(program):
    path = Path(program).expanduser()
    return path.is_file() and os.access(path, os.X_OK)


def find_rosemary(settings):
    """The Rosemary program: the one chosen in settings, else Quadra's own
    newest runner, else Butter's newest, else the one on PATH."""
    chosen = (settings.get("rosemary_program") or "").strip()
    if chosen:
        path = Path(chosen).expanduser()
        if not path.is_file() or not os.access(path, os.X_OK):
            raise RosemaryError(f"{path} is not an executable file. Choose another Rosemary program in "
                                "Settings → Environment.", settings=True)
        return path
    override = os.environ.get("QUADRA_ROSEMARY", "").strip()
    if override:
        # For one run, e.g. a build from the fork: nothing in the settings changes.
        path = Path(override).expanduser()
        if not path.is_file() or not os.access(path, os.X_OK):
            raise RosemaryError(f"QUADRA_ROSEMARY points to {path}, which is not an executable file.")
        return path
    from . import install as rosemary_install
    # The build chosen in settings first; any other installed one still runs the game.
    chosen_kind = rosemary_install.normalize_source(settings.get("rosemary_source"))
    available = quadra_runners(chosen_kind) or quadra_runners() or runners()
    if available:
        return available[0][1]
    found = shutil.which("rosemary")
    if found:
        return Path(found)
    raise RosemaryError("Rosemary is not installed. Quadra installs it for you: press Install in "
                        "Settings → Environment.", settings=True)


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

def refresh_rate(text):
    """`text` as Rosemary reads ROSEMARY_SCREEN_REFRESH_RATE (hertz above zero), else ""."""
    import math
    text = str(text or "").strip()
    if not text or "_" in text:
        return ""
    try:
        value = float(text)
    except ValueError:
        return ""
    return text if math.isfinite(value) and value > 0 else ""


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
    for key, variable, allowed in (("rosemary_presentation_mode", "ROSEMARY_METAL_PRESENTATION_MODE", PRESENTATION_MODES),
                                   ("rosemary_decorations", "ROSEMARY_DECORATIONS", DECORATIONS),
                                   ("rosemary_menu", "ROSEMARY_MENU", MENUS),
                                   ("rosemary_command_key", "ROSEMARY_COMMAND_KEY", COMMAND_KEYS)):
        value = settings.get(key, "")
        if value and value in allowed:
            env[variable] = value
    if settings.get("hide_menu_bar", False):
        # The same setting that hides Darling's menu strip ("Roblox, Edit, Window...").
        env["ROSEMARY_MENU"] = "none"
    rate = refresh_rate(settings.get("rosemary_refresh_rate", ""))
    if rate:
        env["ROSEMARY_SCREEN_REFRESH_RATE"] = rate
    for key, variable in (("rosemary_confine_pointer", "ROSEMARY_CONFINE_POINTER"),
                          ("rosemary_control_edit_keys", "ROSEMARY_CONTROL_EDIT_KEYS"),
                          ("rosemary_plain_ctrl_click", "ROSEMARY_PLAIN_CTRL_CLICK")):
        if settings.get(key, False):
            env[variable] = "1"
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


def installed(settings=None):
    """(Build, bundle) of the build to run: the one pinned in settings
    (`rosemary_pinned_build`, the key of an installed build) if it is there,
    else the newest intact installed build, or None."""
    builds = rosemary_client.installed_builds()
    pinned = (settings or {}).get("rosemary_pinned_build") or ""
    if pinned:
        for found in builds:
            if found[0].key == pinned:
                return found
    return builds[0] if builds else None


def previous_build(settings=None):
    """The newest installed build other than the one that runs, or None."""
    current = installed(settings)
    for found in rosemary_client.installed_builds():
        if current is None or found[0].key != current[0].key:
            return found
    return None


def installed_version(settings=None):
    found = installed(settings)
    return (found[0].version or found[0].guid) if found else None


def rosemary_ready(settings):
    """Whether Rosemary and a Roblox client are both installed."""
    try:
        find_rosemary(settings)
    except RosemaryError:
        return False
    return installed(settings) is not None


def ensure_ready(settings, progress, cancel=None, ignore_pin=False):
    """Install what is missing, then bring Roblox up to date: Rosemary itself
    (from its release, checked against its SHA256SUMS) and the macOS client.
    A program chosen by hand in settings is never replaced. progress(fraction, text)."""
    try:
        find_rosemary(settings)
    except RosemaryError:
        if (settings.get("rosemary_program") or "").strip():
            raise
        from . import install as rosemary_install
        try:
            rosemary_install.install_latest(lambda fraction, text: progress(fraction * 0.4, text),
                                            source=rosemary_install.normalize_source(settings.get("rosemary_source")))
        except rosemary_install.InstallError as error:
            raise RosemaryError(str(error)) from error
        progress(0.4, "")
    return ensure_latest(settings, lambda fraction, text: progress(0.4 + 0.6 * fraction, text), cancel, ignore_pin)


def ensure_latest(settings, progress, cancel=None, ignore_pin=False):
    """Install the current build when it is not installed. progress(fraction,
    text). Returns the bundle path. Offline, the installed build is used. A
    build pinned in settings (a rollback) stays unless `ignore_pin`."""
    cancel = cancel or threading.Event()
    arch = choose_arch(settings)
    if settings.get("rosemary_pinned_build") and not ignore_pin:
        pinned = installed(settings)
        if pinned and pinned[0].key == settings["rosemary_pinned_build"]:
            return pinned[1]
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
    rosemary_client.remove_old(latest.key, keep_previous=1, pinned=settings.get("rosemary_pinned_build") or None)
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
        self.cpu_limited = False
        # The surface of core.RobloxSession the window touches.
        self.shader_preload_progress = None
        self.preload_stage_progress = None
        self.shader_preload_cancel = threading.Event()
        self.web_socket = None
        self.web_user_agent = None
        self.metal_fallback_reason = None
        self.notes = []
        from ..transport_monitor import TransportMonitor
        self.transport_monitor = TransportMonitor()

    def game_log(self):
        """The log that holds the client's own lines (joins, errors): the launch
        log, because the client's output goes to Rosemary's stdout (checked on a
        real run: it holds every FLog line, including the joins)."""
        return self.log_path

    def start(self):
        found = installed(self.settings)
        if found is None:
            raise RosemaryError("Roblox is not installed for Rosemary yet.")
        build, bundle = found
        self.rosemary = find_rosemary(self.settings)
        self.prefix = prefix_path(self.settings)
        if not prefix_created(self.prefix):
            create_prefix(self.rosemary, self.prefix)
        from .. import core
        from . import xr as rosemary_xr
        # VR: the library is injected and Roblox runs on OpenGL (its VR device
        # attaches to that renderer). A reason it cannot start is logged and the
        # game starts without VR, as with Darling.
        vr_session, vr_note = None, None
        try:
            vr_session = rosemary_xr.prepare(self.settings, self.rosemary, core)
        except rosemary_xr.VrUnavailable as error:
            vr_note = "VR: " + str(error).replace("\n", " ")
            self.notes.append(vr_note)
        flags = dict(core.load_fast_flags())
        if vr_session:
            flags.update(vr_session.flags)
        # Always written: a flag from an earlier start (VR's OpenGL) must not stay behind.
        rosemary_client.write_fast_flags(bundle, flags)
        self._apply_mods(build, bundle)
        logs = self.logs_dir or core.LOGS
        logs.mkdir(parents=True, exist_ok=True)
        self.log_path = logs / time.strftime("launch-%Y%m%d-%H%M%S-rosemary.log")
        argv = run_argv(self.rosemary, self.settings, bundle, self.launch_uri)
        cpus = self._cpu_list()
        taskset = shutil.which("taskset") if cpus else None
        if taskset:
            # The limit is in place before Rosemary starts, so no thread escapes it.
            argv = [taskset, "-c", cpus] + argv
        env = run_env(self.settings)
        if vr_session:
            vr_session.apply(env)
        root = self.prefix / "root"
        root.mkdir(parents=True, exist_ok=True)
        with open(self.log_path, "wb") as log_file:
            log_file.write(f"[Quadra Lifecycle] runtime=rosemary rosemary={self.rosemary} "
                           f"prefix={self.prefix} app={bundle}\n".encode())
            if vr_session or vr_note:
                log_file.write(((vr_session.log if vr_session else vr_note) + "\n").encode())
            log_file.flush()
            self.started_at = time.time()
            self.process = subprocess.Popen(argv, cwd=root, env=env, stdin=subprocess.DEVNULL,
                                            stdout=log_file, stderr=subprocess.STDOUT,
                                            start_new_session=True)
        self.cpu_limited = bool(taskset) or bool(cpus and self._limit_cpus_after_start(cpus))

    def _apply_mods(self, build, bundle):
        """The mods folder and presets go into this client's own content, with
        their backups kept per build so one build's files never restore into
        another."""
        if not self.settings.get("enable_custom_mods", True):
            return
        try:
            from .. import core, mods, modguard
            mods.apply_mods(self.settings, bundle / "Contents" / "Resources" / "content",
                            backup_dir=rosemary_client.data_dir() / "mods_backup" / build.key)
            modguard.begin_session(core.CONFIG_DIR, mods.MODS_DIR)
        except Exception as error:  # a broken mod must not stop the game
            log.warning("Could not apply mods: %s", error)

    def _cpu_list(self):
        """The first N cores this process may use, as `taskset -c` writes them, or None."""
        try:
            count = int(self.settings.get("roblox_cpu_limit", 0) or 0)
        except (TypeError, ValueError):
            return None
        allowed = sorted(os.sched_getaffinity(0))
        if count <= 0 or count >= len(allowed):
            return None
        return ",".join(str(cpu) for cpu in allowed[:count])

    def _limit_cpus_after_start(self, cpus):
        """Fallback without `taskset`: every thread started so far gets the limit
        (a thread started later inherits it from its creator)."""
        wanted = {int(cpu) for cpu in cpus.split(",")}
        done = False
        try:
            for tid in os.listdir(f"/proc/{self.process.pid}/task"):
                os.sched_setaffinity(int(tid), wanted)
                done = True
        except (OSError, ValueError):
            pass
        return done

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
        """Ask Rosemary and its children to exit, then insist. Returns once the
        whole process group is gone (or after about 8 seconds)."""
        process = self.process
        if process is None:
            return
        group = process.pid

        def group_alive():
            try:
                os.killpg(group, 0)
                return True
            except (ProcessLookupError, PermissionError):
                return False

        if not group_alive():
            return
        for sig, wait in ((signal.SIGTERM, 4.0), (signal.SIGKILL, 3.0)):
            try:
                os.killpg(group, sig)
            except (ProcessLookupError, PermissionError):
                return
            end = time.monotonic() + wait
            while time.monotonic() < end:
                process.poll()   # reap the leader so the group can empty
                if not group_alive():
                    return
                time.sleep(0.1)


def cookie_files(prefix):
    """The sign-in the client keeps in the prefix (its cookie store), per guest user."""
    return sorted((prefix / "root" / "Users").glob("*/Library/Cookies/com.roblox.*.cookies.tsv"))


def sign_out(prefix):
    """Delete the saved Roblox sign-in. Returns the number of files removed."""
    removed = 0
    for path in cookie_files(prefix):
        try:
            path.unlink()
            removed += 1
        except OSError:
            pass
    return removed
