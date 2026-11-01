#!/usr/bin/env bash
# Builds libQuadraVR.dylib: Quadra's VR device (vr_device.c) and OpenXR session
# (vr_xr.c), unchanged, plus the Rosemary glue, as a dylib that Rosemary
# injects into RobloxPlayer (see rosemary-fork docs/quadra-vr.md).
#
#   DARLING_SYSROOT   the Darwin SDK to compile against (any with the frameworks'
#                     headers; no libc headers are needed). Defaults to the
#                     installed runtime's, else /opt/ciderdarling/*/libexec/darling.
#   QUADRA_BUILD_DIR  output directory (default: ../build)
#   QUADRA_VR_TRACE=1 build libQuadraVR-trace.dylib instead: a copy of vr_device.c,
#                     made in the build directory, that also logs the head pose
#                     Roblox reads from getState (development check only).
set -euo pipefail
here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
project_dir=$(cd -- "$here/.." && pwd -P)
build_dir=${QUADRA_BUILD_DIR:-$project_dir/build}
sysroot=${DARLING_SYSROOT:-}
if [[ -z $sysroot ]]; then
  sysroot=$(python3 -I - "$project_dir/launcher" 2>/dev/null <<'PY' || true
import sys
sys.path.insert(0, sys.argv[1])
from quadra.runtime import resolve_runtime
print(resolve_runtime()['sysroot'])
PY
  )
fi
if [[ -z $sysroot || ! -d $sysroot ]]; then
  sysroot=$(compgen -G '/opt/ciderdarling/*/libexec/darling' | head -n1 || true)
fi
[[ -d $sysroot ]] || { echo "No Darwin sysroot found; set DARLING_SYSROOT." >&2; exit 1; }
mkdir -p "$build_dir"
tmp_output=$(mktemp "$build_dir/.rosemary-vr.XXXXXX")
trap 'rm -f -- "$tmp_output"' EXIT
device_source="$project_dir/vr_device.c"
name=libQuadraVR.dylib
if [[ ${QUADRA_VR_TRACE:-0} == 1 ]]; then
  device_source="$build_dir/vr_device_trace.c"
  name=libQuadraVR-trace.dylib
  python3 -I "$here/make_trace_source.py" "$project_dir/vr_device.c" "$device_source"
fi
# -undefined dynamic_lookup: libSystem and Rosemary's quadra_host_* exports are
# bound by the loader, as the Darling shim is.
clang -target x86_64-apple-darwin -fuse-ld=lld \
  -isysroot "$sysroot" -mmacosx-version-min=11.0 \
  -O2 -fno-omit-frame-pointer -dynamiclib -Werror=incompatible-function-pointer-types \
  -I"$project_dir" \
  -Wl,-undefined,dynamic_lookup \
  -install_name "@rpath/$name" \
  "$device_source" "$project_dir/vr_xr.c" "$here/rosemary_glue.c" \
  -o "$tmp_output"
chmod 755 "$tmp_output"
mv -- "$tmp_output" "$build_dir/$name"
printf 'Built: %s\n' "$build_dir/$name"
