#!/usr/bin/env python3
"""List the symbols a Mach-O imports that neither Darling nor the shim exports.

Darling binds functions lazily, so one that is missing aborts the process on
its first call ("dyld: Symbol not found"); missing_symbols.c was written from
this comparison by hand. Run it on a new Roblox client to see what is left,
before a player finds out.

    tools/check_imports.py RobloxPlayer --sysroot DIR [--shim libQuadraShims.dylib]

Reports undefined symbols (llvm-nm -u) minus the exports of every Mach-O dylib
under the sysroot's /usr/lib and /System/Library, minus the shim's exports.
Data symbols (notification names and constants) are listed apart from
functions, because a missing constant reads as nil rather than aborting. An
Objective-C class that is missing resolves to nil at its first message, so
classes are reported apart too.

Static comparison: it cannot tell a symbol Roblox never calls from one it
calls constantly, so treat the list as a place to look, not a bug count.
"""
import argparse
import os
import subprocess
import sys


def nm(args):
    out = subprocess.run(["llvm-nm", *args], capture_output=True, text=True)
    return [line.split()[-1] for line in out.stdout.splitlines() if line.strip()]


def is_macho(path):
    try:
        with open(path, "rb") as handle:
            return handle.read(4) in (b"\xcf\xfa\xed\xfe", b"\xca\xfe\xba\xbe", b"\xbe\xba\xfe\xca")
    except OSError:
        return False


def exported(path):
    return set(nm(["-gU", path]))


def sysroot_exports(sysroot):
    symbols = set()
    # A sysroot has usr/lib and System/Library; any other directory is walked whole.
    bases = [os.path.join(sysroot, b) for b in ("usr/lib", "System/Library")]
    bases = [b for b in bases if os.path.isdir(b)] or [sysroot]
    for base in bases:
        for root, _, files in os.walk(base):
            for name in files:
                path = os.path.join(root, name)
                if os.path.islink(path) or not is_macho(path):
                    continue
                symbols |= exported(path)
    return symbols


# Resolved at run time by aliases a symbol-table comparison cannot see (libc's
# mem*/str* come from libsystem_platform; Roblox's _mi_* allocator calls are
# bound by the client itself). The client runs today, so these are noise.
KNOWN_RESOLVED = ("_mem", "_str", "_bzero", "_mi_")


def classify(symbol):
    if symbol.startswith("_OBJC_CLASS_$_") or symbol.startswith("_OBJC_METACLASS_$_"):
        return "classes"
    # Mach-O names constants and notification keys in the same namespace as
    # functions; Apple's convention (a leading capital after the underscore,
    # or a trailing Notification/Key) is reliable enough to separate them.
    name = symbol.lstrip("_")
    if name[:1].isupper() and (name.endswith(("Notification", "Key", "Name")) or name.startswith(("k", "GCInput", "GCHaptics"))):
        return "constants"
    if name.startswith("k") and name[1:2].isupper():
        return "constants"
    return "functions"


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("binary")
    parser.add_argument("--sysroot", required=True, help="Darling's libexec/darling directory")
    parser.add_argument("--shim", help="libQuadraShims.dylib, whose exports count as provided")
    parser.add_argument("--extra", action="append", default=[], metavar="DIR",
                        help="more Mach-O files that count as provided, such as the shim's build/frameworks (repeatable)")
    parser.add_argument("--fail", action="store_true", help="exit 1 if any function is missing")
    options = parser.parse_args()

    if not is_macho(options.binary):
        sys.exit(f"{options.binary} is not a Mach-O file")
    provided = sysroot_exports(options.sysroot)
    if options.shim:
        provided |= exported(options.shim)
    for extra in options.extra:
        provided |= sysroot_exports(extra) if os.path.isdir(extra) else exported(extra)
    wanted = set(nm(["-u", options.binary]))
    missing = sorted(s for s in wanted - provided if not s.startswith(KNOWN_RESOLVED))

    groups = {"functions": [], "classes": [], "constants": []}
    for symbol in missing:
        groups[classify(symbol)].append(symbol)
    print(f"{len(wanted)} imported, {len(wanted) - len(missing)} provided, {len(missing)} missing")
    for kind, symbols in groups.items():
        print(f"\n{kind} ({len(symbols)}):")
        for symbol in symbols:
            print("  " + symbol)
    if options.fail and groups["functions"]:
        sys.exit(1)


if __name__ == "__main__":
    main()
