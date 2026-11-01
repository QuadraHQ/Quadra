#!/usr/bin/env bash
# QUADRA_REFRESH_RATE in real Darling: isolated prefix, disposable X server.
set -euo pipefail
project_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)
test_dir=$(mktemp -d /tmp/quadra-refresh.XXXXXX)
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
cleanup() {
  DPREFIX="$test_prefix" darling shutdown >/dev/null 2>&1 || true
  [[ -z ${xvfb_pid:-} ]] || kill "$xvfb_pid" 2>/dev/null || true
  rm -rf -- "$test_dir" 2>/dev/null || true
}
trap cleanup EXIT
clang -target x86_64-apple-darwin -fuse-ld=lld -isysroot "$sysroot" -mmacosx-version-min=11.0 \
  -fobjc-exceptions "$project_dir/tests/darling_refresh_rate_test.m" \
  -framework AppKit -framework Foundation -o "$test_dir/refresh-test"
Xvfb -displayfd 3 -screen 0 800x600x24 -ac -nolisten tcp 3>"$test_dir/display" >"$test_dir/xvfb.log" 2>&1 &
xvfb_pid=$!
for ((attempt = 0; attempt < 100; attempt++)); do
  [[ ! -s "$test_dir/display" ]] || break
  kill -0 "$xvfb_pid" 2>/dev/null || { cat "$test_dir/xvfb.log"; exit 1; }
  sleep 0.05
done
DISPLAY=":$(cat "$test_dir/display")" DPREFIX="$test_prefix" timeout 90s darling shell /bin/bash -c '
  unset WAYLAND_DISPLAY QUADRA_WAYLAND_SOCKET
  export QUADRA_WAYLAND=0 QUADRA_REFRESH_RATE=144
  export DYLD_FORCE_FLAT_NAMESPACE=1
  export DYLD_INSERT_LIBRARIES="/Volumes/SystemRoot$1/libQuadraShims.dylib"
  exec "/Volumes/SystemRoot$2/refresh-test"
' refresh-test "$test_build" "$test_dir" >"$test_dir/test.log" 2>&1 || { cat "$test_dir/test.log"; exit 1; }
grep -E '^(fps=|ok$|FAIL)' "$test_dir/test.log"
