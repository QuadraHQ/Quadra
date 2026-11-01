#!/usr/bin/env bash
# Runs Roblox on Rosemary with libQuadraVR.dylib injected, against the VR
# emulator (Monado, headless), and checks the launch log. Nothing appears on
# your desktop: Roblox gets a private Xvfb, SDL's offscreen driver, or a private
# virtual kwin_wayland. Needs no sudo and does not touch the live Rosemary
# prefix or any saved sign-in: it works on a copy of the app and a new prefix
# under a temporary directory, and removes both (KEEP=1 keeps them).
#
#   test_rosemary_vr.sh [SECONDS]            (default 30; 900 frames take 12.5 s at 72 Hz)
#
#   ROSEMARY            the rosemary binary (default: the fork checkout's release build)
#   ROBLOX_APP          a Roblox.app to copy (default: the newest one Quadra installed for Rosemary)
#   VR_EMULATOR         the emulator directory (default ~/.local/share/quadrahq/vr-emulator)
#   DISPLAY_MODE        offscreen (default; SDL offscreen driver, EGL on the host GPU),
#                       xvfb (private Xvfb, SDL_VIDEO_FORCE_EGL=1, Mesa software GL),
#                       wayland (private virtual kwin_wayland, Mesa software GL)
#   SCREENSHOT=file.png (xvfb mode) save the mirrored eyes 14 s in
#   KEEP=1              keep the temporary directory
set -uo pipefail
here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
seconds=${1:-30}
rosemary=${ROSEMARY:-$HOME/Documents/QuadraHQ/rosemary-fork/target/release/rosemary}
emulator=${VR_EMULATOR:-$HOME/.local/share/quadrahq/vr-emulator}
mode=${DISPLAY_MODE:-offscreen}
app_source=${ROBLOX_APP:-$(compgen -G "$HOME/.local/share/quadrahq/rosemary/versions/*/Roblox.app" | tail -n1 || true)}
[[ -x $rosemary ]] || { echo "No rosemary binary at $rosemary (set ROSEMARY)." >&2; exit 2; }
[[ -x $emulator/run-service.sh ]] || { echo "No VR emulator at $emulator (set VR_EMULATOR)." >&2; exit 2; }
[[ -d $app_source ]] || { echo "No Roblox.app to copy (set ROBLOX_APP)." >&2; exit 2; }

work=$(mktemp -d /tmp/quadra-rosemary-vr.XXXXXX)
started_service=0
session=
cleanup() {
  [[ -n $session ]] && pkill -KILL -s "$session" 2>/dev/null
  ((started_service)) && "$emulator/run-service.sh" stop >/dev/null 2>&1
  if [[ ${KEEP:-0} == 1 ]]; then echo "Kept $work"; else rm -rf -- "$work"; fi
}
trap cleanup EXIT

echo "Building libQuadraVR-trace.dylib (vr_device.c with a getState log line) ..."
QUADRA_BUILD_DIR=$work/build QUADRA_VR_TRACE=1 "$here/build_rosemary_vr.sh" >/dev/null || exit 2
echo "Copying the app and making a prefix ..."
cp -a -- "$app_source" "$work/Roblox.app" || exit 2
python3 -I - "$work/Roblox.app/Contents/MacOS/ClientSettings/ClientAppSettings.json" <<'PY'
import json, os, sys
path = sys.argv[1]
os.makedirs(os.path.dirname(path), exist_ok=True)
flags = json.load(open(path)) if os.path.exists(path) else {}
flags["FFlagDebugGraphicsPreferOpenGL"] = "True"   # the VR device attaches to DeviceGL
flags["FFlagDebugGraphicsDisableMetal"] = "True"
json.dump(flags, open(path, "w"), indent=2)
PY
"$rosemary" prefix create "$work/prefix" --no-base >/dev/null || exit 2

if ! "$emulator/run-service.sh" status 2>/dev/null | grep -q '^running'; then
  IPC_LOG=trace XRT_COMPOSITOR_LOG=trace "$emulator/run-service.sh" start >/dev/null || exit 2
  started_service=1
  sleep 2
fi

log=$work/launch.log
# What the Quadra launcher has to set (see rosemary-fork docs/quadra-vr.md):
guest_environ=$'QUADRA_VR=openxr\nQUADRA_XR_RUNTIME_JSON='"$emulator/monado_openxr.json"
common=(ROSEMARY_PREFIX="$work/prefix"
        ROSEMARY_DYLD_INSERT_LIBRARIES="$work/build/libQuadraVR-trace.dylib"
        ROSEMARY_DYLD_INSERT_FORCE=1
        ROSEMARY_ENVIRON="$guest_environ")
software=(__EGL_VENDOR_LIBRARY_FILENAMES=/usr/share/glvnd/egl_vendor.d/50_mesa.json LIBGL_ALWAYS_SOFTWARE=1)
case $mode in
  offscreen) display_env=(SDL_VIDEO_DRIVER=offscreen) ;;
  xvfb) display_env=(SDL_VIDEO_DRIVER=x11 SDL_VIDEO_FORCE_EGL=1 "${software[@]}") ;;
  wayland) display_env=(SDL_VIDEO_DRIVER=wayland "${software[@]}") ;;
  *) echo "DISPLAY_MODE must be offscreen, xvfb or wayland." >&2; exit 2 ;;
esac

export work_app=$work/Roblox.app
echo "Running Roblox for ${seconds}s ($mode) ..."
setsid bash -c '
  seconds=$1; log=$2; mode=$3; shot=$4; rosemary=$5; shift 5
  unset WAYLAND_DISPLAY DISPLAY
  if [[ $mode == wayland ]]; then
    socket=quadra-vr-test-$$
    KWIN_COMPOSE=Q kwin_wayland --virtual --socket "$socket" --width 1280 --height 800 \
      --no-lockscreen --no-global-shortcuts >/dev/null 2>&1 &
    helper=$!
    for _ in $(seq 100); do [[ -S ${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/$socket ]] && break; sleep 0.1; done
    export WAYLAND_DISPLAY=$socket
  else
    Xvfb -screen 0 1280x800x24 -nolisten tcp -displayfd 3 3>"$log.display" >/dev/null 2>&1 &
    helper=$!
    for _ in $(seq 50); do [[ -s $log.display ]] && break; sleep 0.1; done
    export DISPLAY=:$(<"$log.display")
    if [[ -n $shot ]]; then ( sleep 14; import -display "$DISPLAY" -window root "$shot" 2>/dev/null ) & fi
  fi
  timeout -s TERM -k 3 "$seconds" env "$@" "$rosemary" run "$work_app" >"$log" 2>&1
  kill "$helper" 2>/dev/null
' _ "$seconds" "$log" "$mode" "${SCREENSHOT:-}" "$rosemary" "${common[@]}" "${display_env[@]}" &
session=$!
wait "$session" 2>/dev/null
sleep 1
pkill -KILL -s "$session" 2>/dev/null
session=

# --- checks -----------------------------------------------------------------
failed=0
check() { # description, pattern (extended regex, searched in the launch log)
  if grep -Eq -- "$2" "$log"; then printf 'PASS  %s\n' "$1"; else printf 'FAIL  %s\n' "$1"; failed=1; fi
}
check "injected into the hardened client and patched its graphics classes" 'Quadra VR\] Graphics device VR slot'
check "client's DebugDeviceVR attached to DeviceGL" "Graphics device reports the VR device"
check "OpenXR session created" 'OpenXR session created: [0-9]+x[0-9]+ per eye'
check "session reached FOCUSED" "OpenXR session focused"
check "frame loop paced by the runtime (900-frame report)" '\] 900 frames: [0-9.]+ fps at [0-9]+ Hz'
check "Roblox read a valid head pose through getState" 'trace\] getState #[0-9]+: head valid=1'
if ((seconds < 14)); then echo "NOTE  run at least 14 s to see the 900-frame report"; fi
grep -E 'Quadra VR' "$log" | grep -vE 'trace\] getState #[0-9]{2,}' | cut -c1-200 | head -30
if ((started_service)); then
  service_log=$emulator/logs/service.log
  echo "Runtime side (Monado service trace, this run only):"
  for call in compositor_predict_frame compositor_begin_frame compositor_layer_sync swapchain_acquire_image \
              swapchain_release_image; do
    printf '  %-28s %s\n' "$call" "$(grep -c "Dispatching $call\$" "$service_log")"
  done
  grep -q 'Dispatching compositor_layer_sync' "$service_log" || { echo "FAIL  the runtime saw no xrEndFrame"; failed=1; }
fi
if [[ -n ${SCREENSHOT:-} && -s $SCREENSHOT ]]; then echo "Screenshot: $SCREENSHOT"; fi
exit $failed
