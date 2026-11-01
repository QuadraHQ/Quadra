#!/usr/bin/env bash
# GCController bridge in real Darling, against a virtual gamepad on /dev/uinput.
# Isolated prefix; nothing is installed. Skips when /dev/uinput is not writable.
set -euo pipefail
project_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)
test_dir=$(mktemp -d /tmp/quadra-gamepad.XXXXXX)
test_prefix="$test_dir/prefix"
test_build=${QUADRA_BUILD_DIR:-$project_dir/build}
sysroot=${DARLING_SYSROOT:-}
if [[ -z $sysroot ]]; then
  sysroot=$(python3 - "$project_dir/launcher" <<'PY'
import sys
sys.path.insert(0, sys.argv[1])
from quadra.darling.runtime import resolve_runtime
print(resolve_runtime()['sysroot'])
PY
  )
fi
[[ -w /dev/uinput ]] || { echo "darling gamepad test: skipped (no /dev/uinput)"; exit 0; }
cleanup() {
  exec 7>&- 2>/dev/null || true
  DPREFIX="$test_prefix" darling shutdown >/dev/null 2>&1 || true
  [[ -z ${pad_pid:-} ]] || kill "$pad_pid" 2>/dev/null || true
  rm -rf -- "$test_dir" 2>/dev/null || true
}
trap cleanup EXIT
clang -target x86_64-apple-darwin -fuse-ld=lld -isysroot "$sysroot" -mmacosx-version-min=11.0 \
  -fobjc-exceptions "$project_dir/tests/darling_gamepad_test.m" \
  -framework Foundation -framework GameController -Wl,-undefined,dynamic_lookup -o "$test_dir/gamepad-test"

wait_for_line() { # file pattern seconds
  for ((i = 0; i < $3 * 10; i++)); do
    grep -q -- "$2" "$1" 2>/dev/null && return 0
    sleep 0.1
  done
  echo "timed out waiting for '$2'"; cat "$test_dir/test.log"; return 1
}

DPREFIX="$test_prefix" timeout 120s darling shell /bin/bash -c '
  unset WAYLAND_DISPLAY DISPLAY
  export DYLD_FORCE_FLAT_NAMESPACE=1
  export DYLD_INSERT_LIBRARIES="/Volumes/SystemRoot$1/libQuadraShims.dylib"
  exec "/Volumes/SystemRoot$2/gamepad-test"
' gamepad-test "$test_build" "$test_dir" >"$test_dir/test.log" 2>&1 &
wait_for_line "$test_dir/test.log" WAITING 90

mkfifo "$test_dir/pad.in"
python3 -I "$project_dir/tests/uinput_gamepad.py" <"$test_dir/pad.in" >"$test_dir/pad.log" &
pad_pid=$!
exec 7>"$test_dir/pad.in"
wait_for_line "$test_dir/test.log" CONNECTED 30
{ echo "key 304 1"; echo "abs 0 32767"; echo "abs 1 -32768"; echo sync
  echo "abs 2 1023"; echo "abs 16 -1"; echo sync; } >&7
wait_for_line "$test_dir/test.log" INPUT 30
echo quit >&7
wait_for_line "$test_dir/test.log" '^ok$' 30
tail -n 1 "$test_dir/test.log"
