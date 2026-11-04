"""The files Quadra installs into the Darling prefix: frameworks, and patched runtime and
host-library bridges, with stamps that tell Quadra's copies from the user's."""

import hashlib
import json
import logging
import os
import re
import shutil
import stat
import struct
import subprocess
import tempfile
from pathlib import Path

from ..i18n import _
from . import darling_server, files, locations, process_control

__all__ = [
    "PREFIX_STAMPS",
    "_host_libraries",
    "_initializer_offset",
    "_install_framework",
    "_install_prefix_files",
    "_missing_frameworks",
    "_owned_prefix_copy",
    "_patched_ffmpeg_bridges",
    "_patched_kqueue_runtime",
    "_prefix_stamp",
    "_prefix_stamps",
    "_record_prefix_stamps",
    "_replace_prefix_file",
    "_restore_runtime_library",
    "_sha256",
    "_unknown_prefix_overrides",
    "prepare_prefix",
]


def _missing_frameworks():
    """Stub frameworks the prefix lacks, or holds in an older build."""
    relative = Path("System/Library/Frameworks")
    missing = []
    for name in locations.FRAMEWORKS:
        if (locations.DARLING_SYSROOT / relative / f"{name}.framework").exists():
            continue  # Darling has the real one
        binary = Path(f"{name}.framework") / "Versions" / "A" / name
        try:
            if (locations.DARLING_PREFIX / relative / binary).read_bytes() == (
                locations.FRAMEWORKS_BUILD / binary
            ).read_bytes():
                continue
        except OSError:
            pass
        missing.append(name)
    return missing


def _install_framework(name):
    """Put the fresh build of a stub framework into the prefix. Copying it
    over the old copy failed on the old copy's symlinks (File exists), so it
    is copied next to it and swapped in."""
    frameworks = locations.DARLING_PREFIX / "System" / "Library" / "Frameworks"
    target = frameworks / f"{name}.framework"
    staged = frameworks / f".{name}.framework.new"
    old = frameworks / f".{name}.framework.old"
    frameworks.mkdir(parents=True, exist_ok=True)
    for leftover in (staged, old):  # from an interrupted earlier run
        if leftover.is_dir() and not leftover.is_symlink():
            shutil.rmtree(leftover)
    shutil.copytree(
        locations.FRAMEWORKS_BUILD / f"{name}.framework", staged, symlinks=True
    )
    if target.exists() or target.is_symlink():
        target.rename(old)
    staged.rename(target)
    shutil.rmtree(old, ignore_errors=True)


def prepare_prefix(env):
    """Puts the stub frameworks and repaired runtime libraries into the
    Darling prefix. Its system folders belong to root, so programs inside
    Darling cannot write there; the files go straight into the prefix's
    upper layer (~/.darling) while Darling is stopped, then Darling sees them
    on its next start. Returns lines for the launch log."""
    frameworks = _missing_frameworks()
    bridges = _patched_ffmpeg_bridges()
    kqueue_libraries = _patched_kqueue_runtime()
    if frameworks or bridges or kqueue_libraries:
        _install_prefix_files(env, frameworks, bridges, kqueue_libraries)
    unknown = _unknown_prefix_overrides()
    if not unknown:
        return []
    note = (
        "Prefix libraries not made by Quadra hide the selected runtime's own and were left "
        f"unchanged (remove them if they came from an older Darling): {', '.join(unknown)}"
    )
    logging.getLogger("quadra").warning(note)
    return [note]


def _install_prefix_files(env, frameworks, bridges, kqueue_libraries):
    if not locations.DARLING_PREFIX.is_dir():
        # Let Darling create the prefix first.
        try:
            subprocess.run(
                [locations.DARLING_EXECUTABLE, "shell", "true"],
                env=env,
                stdin=subprocess.DEVNULL,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                timeout=300,
            )
        except (OSError, subprocess.SubprocessError):
            pass
        if not locations.DARLING_PREFIX.is_dir():
            raise RuntimeError(
                _(
                    "Darling could not create its prefix in {path}",
                    path=locations.DARLING_PREFIX,
                )
            )
    if process_control.darlingserver_running():
        darling_server.restart_darling()
    for name in frameworks:
        _install_framework(name)
    from ..darling import patches as darling_patches

    stamps = {}
    for relative, path in [*bridges, *kqueue_libraries]:
        target = locations.DARLING_PREFIX / relative
        if target.is_symlink():
            continue
        if path is None:
            _restore_runtime_library(relative)
        elif not target.exists() and (relative, path) in kqueue_libraries:
            status = darling_patches.install_sparse_map_copy(
                path, target, library=relative.as_posix()
            )
            logging.getLogger("quadra").info("Darling kqueue map: %s", status)
        else:
            # Planning found no file there or a copy this launcher made.
            _replace_prefix_file(path, target)
        stamps[relative.as_posix()] = _prefix_stamp(relative)
    _record_prefix_stamps(stamps)


# Copies the launcher put into a prefix, by prefix and macOS path: the sha256
# of the selected runtime's file each was made from and of the copy itself.
# A runtime update changes the first, so the copy is made again from the new
# file instead of hiding it; a prefix file without a record is someone else's.
PREFIX_STAMPS = locations.CACHE_DIR / "prefix-libraries.json"


def _sha256(data):
    return hashlib.sha256(data).hexdigest()


def _prefix_stamps():
    try:
        if PREFIX_STAMPS.stat().st_size > 1 << 20:
            return {}
        entries = json.loads(PREFIX_STAMPS.read_text()).get(
            str(locations.DARLING_PREFIX.resolve())
        )
        return entries if isinstance(entries, dict) else {}
    except (OSError, ValueError, AttributeError, RuntimeError):
        return {}


def _prefix_stamp(relative):
    """The record for a prefix file now in place, or None when there is none."""
    try:
        return {
            "source": _sha256((locations.DARLING_SYSROOT / relative).read_bytes()),
            "installed": _sha256((locations.DARLING_PREFIX / relative).read_bytes()),
        }
    except OSError:
        return None


def _record_prefix_stamps(changes):
    """Update records ({macOS path: record or None}); unchanged ones stay."""
    try:
        data = json.loads(PREFIX_STAMPS.read_text())
        data = data if isinstance(data, dict) else {}
    except (OSError, ValueError):
        data = {}
    try:
        key = str(locations.DARLING_PREFIX.resolve())
    except (OSError, RuntimeError):
        return
    entries = data.get(key) if isinstance(data.get(key), dict) else {}
    updated = dict(entries)
    for relative, record in changes.items():
        if record is None:
            updated.pop(relative, None)
        else:
            updated[relative] = record
    if updated == entries:
        return
    data[key] = updated
    try:
        files._write_atomically(PREFIX_STAMPS, json.dumps(data, indent=2) + "\n")
    except OSError as error:
        logging.getLogger("quadra").warning(
            "Could not record prefix libraries: %s", error
        )


def _owned_prefix_copy(relative, data, source_data, stamps, recognized):
    """Whether the prefix file holding `data` may be replaced: a copy this
    launcher recorded, an unchanged copy of the runtime's own file (as in a
    rootless prefix), or a repair only this launcher makes (copies from
    releases before the records). Anything else is a custom override."""
    record = stamps.get(relative)
    if isinstance(record, dict) and record.get("installed") == _sha256(data):
        return True
    return data == source_data or recognized(data)


def _replace_prefix_file(source, target):
    """Atomically put a copy of `source` at `target` (Darling is stopped)."""
    target.parent.mkdir(parents=True, exist_ok=True)
    data = Path(source).read_bytes()
    fd, temporary = tempfile.mkstemp(prefix=f".{target.name}.", dir=target.parent)
    try:
        with os.fdopen(fd, "wb") as output:
            os.fchmod(output.fileno(), stat.S_IMODE(Path(source).stat().st_mode))
            output.write(data)
            output.flush()
            os.fsync(output.fileno())
        os.replace(temporary, target)
    finally:
        Path(temporary).unlink(missing_ok=True)


def _restore_runtime_library(relative):
    """Undo this launcher's copy of a library the runtime now provides as is."""
    target = locations.DARLING_PREFIX / relative
    if locations.NOROOT_LIB:
        # A rootless prefix is a full copy of the macOS root: copy it back.
        _replace_prefix_file(locations.DARLING_SYSROOT / relative, target)
    else:
        target.unlink(missing_ok=True)  # the overlay shows the runtime's file again


def _unknown_prefix_overrides(limit=4096):
    """Prefix libraries that hide one of the selected runtime's and that this
    launcher did not make (macOS paths), e.g. a hand-copied libsystem_m."""
    if locations.NOROOT_LIB:
        return []  # a rootless prefix is a full copy of the macOS root
    try:
        prefix = locations.DARLING_PREFIX.resolve()
    except (OSError, RuntimeError):
        return []
    stamps = _prefix_stamps()
    found, seen = [], 0
    for top in ("usr/lib", "System/Library/Frameworks/OpenGL.framework"):
        for directory, _directories, names in os.walk(locations.DARLING_PREFIX / top):
            for name in names:
                seen += 1
                if seen > limit:
                    return sorted(found)
                path = Path(directory) / name
                if not name.endswith(".dylib"):
                    continue
                try:
                    relative = path.relative_to(locations.DARLING_PREFIX)
                    if (
                        path.is_symlink()
                        or not path.is_file()
                        or not path.resolve().is_relative_to(prefix)
                        or not (locations.DARLING_SYSROOT / relative).is_file()
                    ):
                        continue
                    record = stamps.get(relative.as_posix())
                    if isinstance(record, dict) and record.get("installed") == _sha256(
                        path.read_bytes()
                    ):
                        continue
                except (OSError, ValueError, RuntimeError):
                    continue
                found.append(relative.as_posix())
    return sorted(found)


def _patched_kqueue_runtime():
    """Stage the known library repair for this prefix from the selected
    runtime's own file, again when that changed; preserve custom overrides.
    Returns (macOS path, staged file) pairs; None restores the runtime's file."""
    from ..darling import patches as darling_patches

    libraries = []
    stamps, current = _prefix_stamps(), {}
    # Darling builds contain independent copies of libkqueue. Its active
    # close path lives in libSystem.B; libc also has a private copy.
    for relative in (
        Path("usr/lib/libSystem.B.dylib"),
        Path("usr/lib/system/libsystem_c.dylib"),
    ):
        library = relative.as_posix()
        installed = locations.DARLING_PREFIX / relative
        if installed.is_symlink() or not installed.resolve().is_relative_to(
            locations.DARLING_PREFIX.resolve()
        ):
            continue
        # A rootless prefix holds its own copy of every system library: repair
        # that copy, or these two would come from a newer runtime than the rest.
        source = (
            installed
            if locations.NOROOT_LIB and installed.is_file()
            else locations.DARLING_SYSROOT / relative
        )

        def recognized(value, library=library):
            status = darling_patches.plan_sparse_map_patch(value, library=library)[0]
            # A rootless prefix starts as a copy of a (possibly older) root.
            return status == "already patched" or (
                bool(locations.NOROOT_LIB) and status == "patched"
            )

        try:
            source_data = source.read_bytes() if source.is_file() else None
            data = installed.read_bytes() if installed.exists() else None
            if data is not None and not _owned_prefix_copy(
                library, data, source_data, stamps, recognized
            ):
                continue  # a custom override: reported by _unknown_prefix_overrides
            if source_data is None:
                continue
            status, expected = darling_patches.plan_sparse_map_patch(
                source_data, library=library
            )
            if status != "patched":
                if status != "already patched":
                    logging.getLogger("quadra").debug("Darling kqueue map: %s", status)
                if data is not None and data != source_data:
                    libraries.append(
                        (relative, None)
                    )  # our copy of an older runtime file
                continue
            if data == expected:
                current[library] = {
                    "source": _sha256(source_data),
                    "installed": _sha256(data),
                }
                continue
            staged = locations.NATIVE_BUILD / relative.name
            staged_status = darling_patches.install_sparse_map_copy(
                source, staged, library=library
            )
            if staged_status not in ("patched", "already patched"):
                continue
            staged_data = staged.read_bytes()
            if (
                staged_data != expected
                or darling_patches.plan_sparse_map_patch(staged_data, library=library)[
                    0
                ]
                != "already patched"
            ):
                continue
            libraries.append((relative, staged))
        except OSError as error:
            logging.getLogger("quadra").warning(
                "Could not stage Darling kqueue repair: %s", error
            )
    _record_prefix_stamps(current)
    return libraries


def _initializer_offset(data):
    """File offset of the `_initializer` function in the x86_64 slice of a
    (fat) Mach-O library, or None."""
    slice_offset = 0
    if data[:4] == b"\xca\xfe\xba\xbe":
        for index in range(struct.unpack_from(">I", data, 4)[0]):
            cputype, _sub, offset, _size, _align = struct.unpack_from(
                ">5I", data, 8 + index * 20
            )
            if cputype == 0x01000007:
                slice_offset = offset
                break
        else:
            return None
    if data[slice_offset : slice_offset + 4] != b"\xcf\xfa\xed\xfe":
        return None
    ncmds = struct.unpack_from("<I", data, slice_offset + 16)[0]
    position = slice_offset + 32
    segments, symtab = [], None
    for _command in range(ncmds):
        command, size = struct.unpack_from("<II", data, position)
        if command == 0x19:  # LC_SEGMENT_64
            segments.append(struct.unpack_from("<4Q", data, position + 24))
        elif command == 0x2:  # LC_SYMTAB
            symtab = struct.unpack_from("<4I", data, position + 8)
        position += size
    if not symtab:
        return None
    symoff, nsyms, stroff, _strsize = symtab
    for index in range(nsyms):
        strx, _type, _sect, _desc, value = struct.unpack_from(
            "<IBBHQ", data, slice_offset + symoff + index * 16
        )
        name_start = slice_offset + stroff + strx
        if data[name_start : data.index(b"\0", name_start)] != b"_initializer":
            continue
        for vmaddr, vmsize, fileoff, _filesize in segments:
            if vmaddr <= value < vmaddr + vmsize:
                return slice_offset + fileoff + value - vmaddr
    return None


def _host_libraries():
    try:
        output = subprocess.run(
            ["ldconfig", "-p"], capture_output=True, text=True
        ).stdout
    except OSError:
        return set()
    return {line.split()[0] for line in output.splitlines()[1:] if line.strip()}


def _patched_ffmpeg_bridges():
    """Darling's bridges to host libraries (/usr/lib/native/libav*.dylib and
    others) load one exact host version from an initializer, and the game
    exits when it is missing ("Cannot load libavformat.so.60"). Roblox does
    not need these, so when the host has another version (or none, as in the
    Flatpak), the prefix gets copies whose initializer returns right away.
    Copies are made from the selected runtime's file, and made again when it
    changed. Returns (path in the macOS root, patched file) pairs to install;
    None restores the runtime's own file."""
    host = None
    patched = []
    stamps, current = _prefix_stamps(), {}

    def initializer(data):
        try:
            offset = _initializer_offset(data)
        except (ValueError, IndexError, struct.error):
            return None
        return data[offset] if offset is not None and offset < len(data) else None

    def recognized(data):
        # 0xC3 (ret) is this launcher's patch; a rootless prefix starts as a
        # copy of a (possibly older) root with the stock 0x55 (push %rbp).
        return initializer(data) == 0xC3 or (
            bool(locations.NOROOT_LIB) and initializer(data) == 0x55
        )

    for relative in map(Path, locations.NATIVE_LIBS):
        installed = locations.DARLING_PREFIX / relative
        name = relative.stem
        try:
            if installed.is_symlink() or not installed.resolve().is_relative_to(
                locations.DARLING_PREFIX.resolve()
            ):
                continue
            source_data = (locations.DARLING_SYSROOT / relative).read_bytes()
            data = installed.read_bytes() if installed.exists() else None
        except (OSError, RuntimeError):
            continue
        if data is not None and not _owned_prefix_copy(
            relative.as_posix(), data, source_data, stamps, recognized
        ):
            continue  # a custom override: reported by _unknown_prefix_overrides
        wanted = source_data
        library = re.search(rb"%s\.so\.\d+" % name.encode(), source_data)
        host = _host_libraries() if host is None else host
        if (
            library
            and library.group().decode() not in host
            and initializer(source_data) == 0x55
        ):
            data_patched = bytearray(source_data)
            data_patched[_initializer_offset(source_data)] = 0xC3  # ret
            wanted = bytes(data_patched)
        if data == wanted:
            if wanted != source_data:
                current[relative.as_posix()] = {
                    "source": _sha256(source_data),
                    "installed": _sha256(data),
                }
            continue
        if wanted == source_data:
            if data is not None:
                patched.append((relative, None))  # our copy of an older runtime file
            continue
        try:
            locations.NATIVE_BUILD.mkdir(parents=True, exist_ok=True)
            (locations.NATIVE_BUILD / f"{name}.dylib").write_bytes(wanted)
            (locations.NATIVE_BUILD / f"{name}.dylib").chmod(0o755)
        except OSError as error:
            logging.getLogger("quadra").warning(
                "Could not stage %s: %s", relative, error
            )
            continue
        patched.append((relative, locations.NATIVE_BUILD / f"{name}.dylib"))
    _record_prefix_stamps(current)
    return patched
