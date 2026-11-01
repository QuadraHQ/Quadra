# Rosemary

Everything native that only the Rosemary runtime uses. The Python that goes with it is in
`launcher/quadra/rosemary/`.

| Folder | What |
|---|---|
| `vr/` | VR for Roblox under Rosemary. `build_rosemary_vr.sh` builds `libQuadraVR.dylib` from `../vr/` plus `rosemary_glue.c`, which Rosemary injects into RobloxPlayer. This needs Quadra's build of Rosemary ([QuadraHQ/rosemary-fork](https://github.com/QuadraHQ/rosemary-fork), branch `quadra-vr`); the launcher says so when another Rosemary is selected. `test_rosemary_vr.sh` is a development check |

Rosemary itself is not built here. The launcher downloads a release of it
(`launcher/quadra/rosemary/install.py`).
