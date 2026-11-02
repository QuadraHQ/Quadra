"""Finding, stopping and prioritising Roblox and Darling processes, limited to the
selected prefix."""

import os
import re
import resource
import shutil
import signal
import subprocess
import time
from pathlib import Path

from ..darling.rootless_scope import rootless_process_in_prefix
from . import locations

__all__ = [
    "_user_commands",
    "_user_processes",
    "roblox_pids",
    "limit_roblox_cpus",
    "_darlingservers",
    "_foreign_darlingservers",
    "_server_for_prefix",
    "_mountinfo_prefix",
    "_process_prefix",
    "_process_in_prefix",
    "_mount_namespace",
    "_prefix_namespaces",
    "_roblox_process_in_prefix",
    "_darling_processes",
    "_container_processes",
    "_orphaned_darling_processes",
    "_terminate",
    "_terminate_scoped",
    "_signal_scoped",
    "_terminate_roblox",
    "_kill_crash_handlers",
    "_terminate_frontend",
    "clear_orphaned_darling",
    "SERVER_NICE",
    "PLAYER_NICE",
    "_nice_target",
    "_raise_priority",
    "_with_descendants",
    "raise_darling_priority",
    "darlingserver_running",
    "stop_roblox",
    "_process_state",
]


def _user_commands():
    """(pid, argv) of this user's processes."""
    uid = os.getuid()
    for entry in Path("/proc").iterdir():
        if not entry.name.isdigit():
            continue
        try:
            if entry.stat().st_uid != uid:
                continue
            argv = (entry / "cmdline").read_bytes().split(b"\0")
        except OSError:
            continue
        yield int(entry.name), [arg.decode(errors="replace") for arg in argv if arg]


def _user_processes():
    """(pid, command line joined by spaces) of this user's processes."""
    for pid, argv in _user_commands():
        yield pid, " ".join(argv)


def roblox_pids(names=("RobloxPlayer", "RobloxCrashHandler")):
    """Host PIDs of Roblox executables in the selected Darling prefix."""
    namespaces = _prefix_namespaces()
    return [
        pid
        for pid, argv in _user_commands()
        if argv and Path(argv[0]).name in names and _process_in_prefix(pid, namespaces)
    ]


def limit_roblox_cpus(count):
    """Restrict the game's processes to `count` of the cores they may use.
    Returns how many processes were restricted (0 when off or none found)."""
    if not isinstance(count, int) or count <= 0:
        return 0
    limited = 0
    for pid in roblox_pids(("RobloxPlayer",)):
        try:
            allowed = sorted(os.sched_getaffinity(pid))
            if count < len(allowed):
                os.sched_setaffinity(pid, set(allowed[:count]))
            limited += 1
        except (OSError, ValueError):
            continue
    return limited


def _darlingservers():
    """darlingserver processes of our prefix."""
    return [pid for pid, argv in _user_commands() if _server_for_prefix(pid, argv)]


def _foreign_darlingservers():
    """Our prefix's darlingservers started by another runtime installation.
    An update installs a new CiderDarling revision for the same prefix, and
    the old revision's server would go on serving its own libraries.
    darling execs INSTALL_PREFIX/bin/darlingserver next to itself."""
    try:
        expected = (
            Path(shutil.which(locations.DARLING_EXECUTABLE) or locations.DARLING_EXECUTABLE)
            .resolve()
            .parent
            / "darlingserver"
        )
    except (OSError, RuntimeError):
        return []
    if not expected.is_file():
        return []  # another layout (a wrapper): nothing to compare with
    foreign = []
    for pid in _darlingservers():
        try:
            running = os.readlink(f"/proc/{pid}/exe").removesuffix(" (deleted)")
        except OSError:
            continue  # unknown: leave it
        if Path(running) != expected:
            foreign.append(pid)
    return foreign


def _server_for_prefix(pid, argv):
    if len(argv) < 2 or Path(argv[0]).name != "darlingserver":
        return False
    try:
        prefix = Path(argv[1])
        if not prefix.is_absolute():
            prefix = Path(os.readlink(f"/proc/{pid}/cwd")) / prefix
        return prefix.resolve() == locations.DARLING_PREFIX.resolve()
    except (OSError, RuntimeError):
        return False


def _mountinfo_prefix(text):
    """Identify Darling's prefix overlay; ambiguous/missing evidence is None.

    Darling mounts upperdir=prefix,workdir=prefix.workdir on prefix itself.
    A rootless copy has no such evidence, so its orphan scope stays unknown.
    """

    def decode(value):
        return re.sub(r"\\([0-7]{3})", lambda match: chr(int(match.group(1), 8)), value)

    prefixes = set()
    for line in text.splitlines():
        before, separator, after = line.partition(" - ")
        fields, filesystem = before.split(), after.split()
        if (
            not separator
            or len(fields) < 6
            or len(filesystem) < 3
            or filesystem[0] != "overlay"
        ):
            continue
        options = dict(
            option.split("=", 1) for option in filesystem[2].split(",") if "=" in option
        )
        upper, work = options.get("upperdir"), options.get("workdir")
        if upper is None or work is None:
            continue
        upper, work, mounted = decode(upper), decode(work), decode(fields[4])
        if not Path(upper).is_absolute() or work != upper + ".workdir":
            continue
        try:
            prefix = Path(upper).resolve()
            if Path(mounted).resolve() == prefix:
                prefixes.add(prefix)
        except (OSError, RuntimeError):
            pass
    return next(iter(prefixes)) if len(prefixes) == 1 else None


def _process_prefix(pid):
    try:
        return _mountinfo_prefix(
            Path(f"/proc/{pid}/mountinfo").read_text(errors="surrogateescape")
        )
    except OSError:
        return None


def _process_in_prefix(pid, namespaces=()):
    try:
        if Path(f"/proc/{pid}").stat().st_uid != os.getuid():
            return False
        namespace = _mount_namespace(pid)
        if namespace is None:
            return rootless_process_in_prefix(pid, locations.DARLING_PREFIX, locations.NOROOT_LIB)
        if namespace in namespaces:
            return True
        argv = Path(f"/proc/{pid}/cmdline").read_bytes().split(b"\0")
        argv = [argument.decode(errors="replace") for argument in argv if argument]
        return (
            _server_for_prefix(pid, argv)
            or rootless_process_in_prefix(pid, locations.DARLING_PREFIX, locations.NOROOT_LIB)
            or _process_prefix(pid) == locations.DARLING_PREFIX.resolve()
        )
    except (OSError, RuntimeError):
        return False


def _mount_namespace(pid):
    try:
        return os.readlink(f"/proc/{pid}/ns/mnt")
    except OSError:
        return None


def _prefix_namespaces(servers=None):
    """Private mount namespaces of freshly verified selected-prefix servers."""
    host = _mount_namespace(os.getpid())
    if host is None:
        return set()
    namespaces = set()
    for pid in _darlingservers() if servers is None else servers:
        namespace = _mount_namespace(pid)
        if namespace is None or namespace == host:
            continue
        try:
            if Path(f"/proc/{pid}").stat().st_uid != os.getuid():
                continue
            argv = Path(f"/proc/{pid}/cmdline").read_bytes().split(b"\0")
            argv = [argument.decode(errors="replace") for argument in argv if argument]
            if _server_for_prefix(pid, argv) and _mount_namespace(pid) == namespace:
                namespaces.add(namespace)
        except OSError:
            pass
    return namespaces


def _roblox_process_in_prefix(
    pid, namespaces=(), names=("RobloxPlayer", "RobloxCrashHandler")
):
    """Recheck executable name and prefix immediately before a scoped signal."""
    try:
        argv = Path(f"/proc/{pid}/cmdline").read_bytes().split(b"\0")
        name = Path(argv[0].decode(errors="replace")).name if argv[0] else ""
        return name in names and _process_in_prefix(pid, namespaces)
    except OSError:
        return False


def _darling_processes():
    """(pid, mount namespace) of this user's Darling processes: every macOS
    program, Darling's daemons included, runs under the mldr loader."""
    found = []
    for pid, _argv in _user_commands():
        try:
            if os.readlink(f"/proc/{pid}/exe").rsplit("/", 1)[-1] == "mldr":
                found.append((pid, _mount_namespace(pid)))
        except OSError:
            pass
    return found


def _container_processes(servers):
    """Darling processes in the containers of the darlingservers `servers`
    (private mount namespaces, or proven prefix runtime mappings for rootless
    guests). launchd and the daemons may not be server children."""
    namespaces = _prefix_namespaces(servers)
    server_ids = set(servers)
    rootless_selected = bool(locations.NOROOT_LIB) and any(
        pid in server_ids and _server_for_prefix(pid, argv)
        for pid, argv in _user_commands()
    )
    return [
        pid
        for pid, namespace in _darling_processes()
        if namespace in namespaces
        or (
            rootless_selected
            and rootless_process_in_prefix(pid, locations.DARLING_PREFIX, locations.NOROOT_LIB)
        )
    ]


def _orphaned_darling_processes():
    """Orphaned Darling processes proven to belong to the selected prefix."""
    server_commands = [
        (pid, argv)
        for pid, argv in _user_commands()
        if argv and Path(argv[0]).name == "darlingserver"
    ]
    servers = [pid for pid, _argv in server_commands]
    alive = {_mount_namespace(pid) for pid in servers} - {None}
    selected_server_alive = any(
        _server_for_prefix(pid, argv) for pid, argv in server_commands
    )
    try:
        selected = locations.DARLING_PREFIX.resolve()
    except (OSError, RuntimeError):
        return []
    overlay_orphans, rootless_orphans = [], []
    for pid, namespace in _darling_processes():
        if (
            namespace is not None
            and namespace not in alive
            and _process_prefix(pid) == selected
        ):
            overlay_orphans.append(pid)
        elif (
            locations.NOROOT_LIB
            and not selected_server_alive
            and rootless_process_in_prefix(pid, selected, locations.NOROOT_LIB)
        ):
            rootless_orphans.append(pid)
    # A server may have started while the guest mappings were being scanned.
    # A shared namespace cannot distinguish its current guests from orphans.
    if rootless_orphans and darlingserver_running():
        rootless_orphans = []
    return overlay_orphans + rootless_orphans


def _terminate(pids, wait=5.0, *, scope=None):
    """Terminate only verified targets, using stable handles through escalation."""
    return _terminate_scoped(
        pids, wait, scope if scope is not None else _process_in_prefix
    )


def _terminate_scoped(pids, wait, scope):
    """Use stable pidfds and verify scope before signaling; unknowns stay."""
    if not hasattr(os, "pidfd_open") or not hasattr(signal, "pidfd_send_signal"):
        return
    targets = {}
    try:
        for pid in dict.fromkeys(pids):
            try:
                descriptor = os.pidfd_open(pid)
            except OSError:
                continue
            try:
                accepted = scope(pid)
            except BaseException:
                os.close(descriptor)
                raise
            if not accepted:
                os.close(descriptor)
                continue
            targets[pid] = descriptor
            try:
                signal.pidfd_send_signal(descriptor, signal.SIGTERM)
            except OSError:
                pass
        deadline = time.monotonic() + wait
        while time.monotonic() < deadline and any(
            _process_state(pid) not in (None, "Z") for pid in targets
        ):
            time.sleep(0.1)
        for pid, descriptor in targets.items():
            if _process_state(pid) not in (None, "Z"):
                try:
                    signal.pidfd_send_signal(descriptor, signal.SIGKILL)
                except OSError:
                    pass
    finally:
        for descriptor in targets.values():
            os.close(descriptor)


def _signal_scoped(pids, sig, scope):
    """Send one signal through a pidfd after checking the current scope."""
    if not hasattr(os, "pidfd_open") or not hasattr(signal, "pidfd_send_signal"):
        return
    for pid in dict.fromkeys(pids):
        try:
            descriptor = os.pidfd_open(pid)
        except OSError:
            continue
        try:
            if scope(pid):
                try:
                    signal.pidfd_send_signal(descriptor, sig)
                except OSError:
                    pass
        finally:
            os.close(descriptor)


def _terminate_roblox(pids, wait=3):
    namespaces = _prefix_namespaces()
    _terminate(pids, wait, scope=lambda pid: _roblox_process_in_prefix(pid, namespaces))


def _kill_crash_handlers():
    namespaces = _prefix_namespaces()
    _signal_scoped(
        roblox_pids(("RobloxCrashHandler",)),
        signal.SIGKILL,
        lambda pid: _roblox_process_in_prefix(pid, namespaces, ("RobloxCrashHandler",)),
    )


def _terminate_frontend(process):
    """End this session's owned frontend and proven prefix descendants."""
    if process.poll() is not None:
        return
    namespaces = _prefix_namespaces()
    descendants = [
        pid for pid in _with_descendants([process.pid]) if pid != process.pid
    ]
    _terminate(
        descendants, wait=1, scope=lambda pid: _process_in_prefix(pid, namespaces)
    )
    # An unreaped Popen child cannot have its PID reused. Recheck ownership
    # after opening the pidfd; descendants are checked individually above.
    _terminate(
        [process.pid],
        wait=2,
        scope=lambda pid: pid == process.pid and process.poll() is None,
    )
    try:
        process.wait(timeout=2)
    except subprocess.TimeoutExpired:
        pass


def clear_orphaned_darling():
    """End proven selected-prefix leftovers; preserve other/unknown scopes."""
    orphans = _orphaned_darling_processes()
    if orphans:

        def still_orphaned(pid):
            if locations.NOROOT_LIB:
                # Never fall through to broad server/prefix membership if the
                # collected guest has vanished or its PID has been reused.
                return (
                    rootless_process_in_prefix(pid, locations.DARLING_PREFIX, locations.NOROOT_LIB)
                    and not darlingserver_running()
                )
            try:
                process = Path(f"/proc/{pid}")
                if process.stat().st_uid != os.getuid():
                    return False
                if os.readlink(process / "exe").rsplit("/", 1)[-1] != "mldr":
                    return False
                namespace = _mount_namespace(pid)
                live = {
                    _mount_namespace(server)
                    for server, argv in _user_commands()
                    if argv and Path(argv[0]).name == "darlingserver"
                }
                return (
                    namespace is not None
                    and namespace not in live
                    and _process_prefix(pid) == locations.DARLING_PREFIX.resolve()
                )
            except (OSError, RuntimeError):
                return False

        # Scope is rechecked after obtaining the stable pidfd, so a server
        # starting after collection cannot make new rootless guests targets.
        _terminate(orphans, scope=still_orphaned)
    return len(orphans)


# Nice values. Darling runs the game at about -4, while darlingserver,
# Darling's daemons (launchd, notifyd... all "mldr") and the audio player
# inherit the launcher's nice value, +10 when the desktop starts apps niced.
# The game waits on darlingserver for most of its system calls and on the
# player for every audio period; niced below the game, they got about a
# twentieth of its CPU share while it kept the cores busy: frame times of
# seconds, network stalls, crackling and silent audio.
SERVER_NICE = -5
PLAYER_NICE = -11


def _nice_target(nice):
    """`nice`, or the lowest nice value RLIMIT_NICE lets this user set."""
    soft = resource.getrlimit(resource.RLIMIT_NICE)[0]
    return nice if soft == resource.RLIM_INFINITY else max(nice, 20 - soft)


def _raise_priority(pid, nice):
    """Give every thread of `pid` (Linux keeps nice values per thread) the
    nice value `nice` if that is better than its own; True if it was set."""
    try:
        current = os.getpriority(os.PRIO_PROCESS, pid)
        threads = [int(tid) for tid in os.listdir(f"/proc/{pid}/task")]
    except OSError:
        return False
    if nice >= current:
        return False
    done = False
    for tid in threads:
        try:
            os.setpriority(os.PRIO_PROCESS, tid, nice)
            done = True
        except OSError:
            pass
    return done


def _with_descendants(roots):
    """`roots` and every process below them."""
    children = {}
    for entry in Path("/proc").iterdir():
        if entry.name.isdigit():
            try:
                stat = (entry / "stat").read_text()
                parent = int(stat.rsplit(")", 1)[1].split()[1])
            except (OSError, ValueError, IndexError):
                continue
            children.setdefault(parent, []).append(int(entry.name))
    found, pending = [], list(roots)
    while pending:
        pid = pending.pop()
        found.append(pid)
        pending.extend(children.get(pid, []))
    return found


def raise_darling_priority():
    """Raise darlingserver and every process under it (Darling's daemons;
    the game, already better, stays as it is) to SERVER_NICE. Returns how
    many processes changed."""
    target = _nice_target(SERVER_NICE)
    return sum(
        _raise_priority(pid, target) for pid in _with_descendants(_darlingservers())
    )


def darlingserver_running():
    return bool(_darlingservers())


def stop_roblox():
    _terminate_roblox(roblox_pids(), wait=3)


def _process_state(pid):
    try:
        return Path(f"/proc/{pid}/stat").read_text().rsplit(")", 1)[1].split()[0]
    except (OSError, IndexError):
        return None
