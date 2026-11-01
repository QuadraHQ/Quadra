# VR device

Roblox's macOS client still contains its VR pipeline but no device that talks to a headset. These
sources add one, using the host's OpenXR runtime. [docs/VR.md](../docs/VR.md) explains how.

| File | What |
|---|---|
| `vr_device.c` | Attaches the client's own `DebugDeviceVR` to `DeviceGL` with a vtable that calls OpenXR |
| `vr_xr.c`, `vr_xr.h` | The OpenXR session: frames, eyes, controllers, vibration |
| `vr_openxr.h` | The part of the OpenXR 1.0 ABI that `vr_xr.c` uses, so the build needs no OpenXR headers |

Both runtimes compile these files unchanged:

- **Darling:** into `libQuadraShims.dylib` (`../darling/shim/`, built by `../build_debug_shim.sh`).
- **Rosemary:** into `libQuadraVR.dylib` (`../rosemary/vr/build_rosemary_vr.sh`).
