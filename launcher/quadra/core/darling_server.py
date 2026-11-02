"""The Darling container: its version and environment, starting and restarting its server,
and keeping its log small."""

import logging
import os
import subprocess

from ..i18n import _
from . import locations, logs, process_control

__all__ = [
    "darling_version",
    "darling_environment",
    "DSERVER_LOG_MAX_BYTES",
    "rotate_dserver_log",
    "clear_stale_darling",
    "restart_darling",
    "WARMUP_OUTPUT_LIMIT",
    "_warmup_output_path",
    "_darling_warmup",
    "_warmup_output",
    "_prepare_darling_server",
]


def darling_version(env=None):
    """What `darling --version` calls itself (a release prints its commit),
    for the launch log: reports from another Darling build than the tested
    one are otherwise hard to tell apart."""
    try:
        result = subprocess.run(
            [locations.DARLING_EXECUTABLE, "--version"],
            capture_output=True,
            text=True,
            timeout=5,
            env=env,
            stdin=subprocess.DEVNULL,
        )
    except (OSError, subprocess.SubprocessError):
        return "unknown"
    lines = (result.stdout or result.stderr).strip().splitlines()
    return lines[0].strip() if lines else "unknown"


def darling_environment(settings=None, notes=None):
    """Environment for every `darling` command the launcher runs.

    darlingserver keeps the environment of the command that started it. An
    inherited DSERVER_LOG_STDERR copied every server message into the launch
    log (one grew to 143 MB), so DSERVER_LOG_* is only kept on request:
    CIDER_KEEP_DSERVER_LOG=1, or CiderDarling's verbose server log setting.
    `notes` collects a line for the launch log about what was dropped.
    """
    env = dict(os.environ)
    env["DPREFIX"] = str(locations.DARLING_PREFIX)
    # Darling's Mesa receives X11 displays; a Wayland session may say otherwise.
    env["EGL_PLATFORM"] = "x11"
    if locations.NOROOT_LIB:
        env["LD_PRELOAD"] = locations.NOROOT_LIB
    if env.get("CIDER_KEEP_DSERVER_LOG") != "1":
        dropped = sorted(name for name in env if name.startswith("DSERVER_LOG_"))
        for name in dropped:
            del env[name]
        if dropped:
            note = (
                f"Ignoring inherited {', '.join(dropped)} for darlingserver "
                "(CIDER_KEEP_DSERVER_LOG=1 keeps them)"
            )
            logging.getLogger("quadra").info(note)
            if notes is not None:
                notes.append(note)
    if (
        settings
        and settings.get("cider_verbose_server_log")
        and locations.DARLING_RUNTIME["runtime"] == "ciderdarling"
    ):
        env["DSERVER_LOG_LEVEL"] = "debug"
    return env


# darlingserver appends to this for as long as the prefix exists.
DSERVER_LOG_MAX_BYTES = 32 * 1024 * 1024


def rotate_dserver_log(limit=DSERVER_LOG_MAX_BYTES):
    """Keep one older dserver.log (dserver.log.old) when it grew past `limit`.
    Only while Darling is stopped: the server holds the file open."""
    log = locations.DARLING_PREFIX / "private" / "var" / "log" / "dserver.log"
    try:
        if log.is_symlink() or not log.is_file() or log.stat().st_size <= limit:
            return False
        if process_control.darlingserver_running():
            return False
        log.replace(log.with_name("dserver.log.old"))
        return True
    except OSError:
        return False


def clear_stale_darling():
    """If the container's init (darlingserver, whose PID is in .init.pid) is
    gone or a zombie, darling refuses to start ("Cannot open mnt namespace
    file"); move its pid file and socket aside so a new server starts. After
    a reboot the PID may belong to an unrelated process: only one of our
    prefix's darlingservers counts as alive."""
    prefix = locations.DARLING_PREFIX
    try:
        pid = int((prefix / ".init.pid").read_text().strip())
    except OSError:
        return
    except ValueError:
        pid = None
    if pid is not None and pid in process_control._darlingservers():
        return
    for name in (".init.pid", ".darlingserver.sock"):
        path = prefix / name
        if path.exists():
            path.rename(prefix / (name + ".stale"))


def restart_darling():
    """Stops the prefix's darlingserver and everything in its container
    (launchd, Darling's daemons, a game still closing), and proven selected-
    prefix leftovers; the next darling command starts them again. The
    daemons are not darlingserver's children: stopping only its descendants
    left them running without a server. Blocks for up to a few seconds: call
    it off the GTK thread."""
    servers = process_control._darlingservers()
    namespaces = process_control._prefix_namespaces(servers)
    processes = set(process_control._with_descendants(servers)) | set(process_control._container_processes(servers))
    processes |= set(process_control._orphaned_darling_processes())
    # The container's programs first, so none of them runs on without its server.
    belongs = lambda pid: process_control._process_in_prefix(pid, namespaces)
    process_control._terminate(sorted(processes - set(servers)), scope=belongs)
    process_control._terminate(servers, scope=belongs)
    clear_stale_darling()


# darlingserver's inherited stdout and stderr (see RobloxSession._start).
WARMUP_OUTPUT_LIMIT = 16384


def _warmup_output_path():
    return locations.LOGS / f"darlingserver-{locations.DARLING_RUNTIME['runtime']}.out"


def _darling_warmup(env):
    """Start the prefix's darlingserver with `darling shell true`; its exit
    status. The server keeps this command's stdout and stderr for its whole
    life, so they go to a file of its own (replaced per warmup: the previous
    server of this runtime is gone by then)."""
    locations.LOGS.mkdir(parents=True, exist_ok=True)
    with open(_warmup_output_path(), "wb") as output:
        return subprocess.run(
            [locations.DARLING_EXECUTABLE, "shell", "true"],
            env=env,
            stdin=subprocess.DEVNULL,
            stdout=output,
            stderr=subprocess.STDOUT,
            timeout=120,
        ).returncode


def _warmup_output():
    """The warmup's output so far, for the launch log ("" when empty)."""
    text = logs._log_tail(_warmup_output_path(), WARMUP_OUTPUT_LIMIT)
    return text if not text or text.endswith("\n") else text + "\n"


def _prepare_darling_server(env, log):
    """Warm up the server, preserving transient retries and known failures."""
    for _attempt in range(2):
        # A first process can fail to check in while the server starts. Keep
        # the server's output separate so it cannot hold the launch log open.
        if process_control.darlingserver_running():
            return
        log.write(b"[Quadra Startup] Darling warmup starting\n")
        log.flush()
        status = _darling_warmup(env)
        output = _warmup_output()
        log.write(output.encode(errors="replace"))
        log.write(f"[Quadra Startup] Darling warmup status={status}\n".encode())
        if status and "Cannot mount overlay: No such device" in output:
            log.write(
                b"[Quadra Startup] OverlayFS unavailable; stopping before Roblox starts\n"
            )
            log.flush()
            raise RuntimeError(
                _(
                    "Darling could not start because the kernel's OverlayFS filesystem is unavailable. "
                    "Roblox has not started.\n\n"
                    "Run sudo modprobe overlay in a terminal, then try launching again. "
                    "If the module is missing after a kernel update, reboot into the installed kernel. "
                    "A custom kernel needs CONFIG_OVERLAY_FS enabled."
                )
            )
