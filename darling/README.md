# Darling

Everything native that only the Darling runtime uses. The Python that goes with it is in
`launcher/quadra/darling/`.

| Folder | What |
|---|---|
| `shim/` | `libQuadraShims.m` and the C, Objective-C and C++ sources beside it. The shim is injected into RobloxPlayer inside Darling: input, cursor, audio, DNS, sockets, Wayland windows, web sign-in, gamepads and fixes for what Darling lacks. `wayland_host.cpp` is the Linux helper behind native Wayland. `../build_debug_shim.sh` builds it all |
| `frameworks/` | Stub frameworks (CoreML, CoreHaptics, DeviceCheck) that RobloxPlayer links and Darling does not have. Built by `../build_debug_shim.sh` |
| `metal-vulkan/` | Quadra M2VK: Roblox's Metal renderer on the host's Vulkan driver. Its [README](metal-vulkan/README.md) covers building and running it |

The OpenXR VR device the shim also contains lives in `../vr/`, because Rosemary builds it too.

`../build_debug_shim.sh`, `../run_debug.sh` and `../patch_startup_throttle.py` stay at the top of the
repository: the installer, the launcher and every package recipe call them there.
