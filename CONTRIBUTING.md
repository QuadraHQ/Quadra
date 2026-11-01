# Contributing to Quadra

Quadra runs the macOS Roblox client on Linux through [Darling](https://www.darlinghq.org). It is a launcher
(Python, GTK 4) plus a shim injected into the game, and optionally a Metal-on-Vulkan renderer. Bug reports,
hardware test results, translations and code are all welcome.

Quadra is under the [GNU GPL v3](LICENSE). By sending a change you agree that it is released under that
license. The notice for Mac O' Blox, the MIT-licensed code Quadra started from, stays in
[LICENSES/MIT-MacOBlox.txt](LICENSES/MIT-MacOBlox.txt).

## Reporting a bug

Use the [bug report form](https://github.com/QuadraHQ/Quadra/issues/new/choose) and attach the
**diagnostics archive from the run that failed** (menu → **Collect diagnostics**). For a crash, turn on
**Settings → Environment → Diagnostics → Crash diagnostics** and **Allow freeze diagnostics** before you start
Roblox. For a freeze, collect while the game is still frozen. Without a log from the failing launch there is
usually nothing to diagnose.

Issues and attachments are public. Look through the archive before you upload it, and never paste your
`.ROBLOSECURITY` cookie, a join link or a token anywhere. Questions and feature ideas go to the
[Discord server](https://discord.gg/W8jXfngc8z). If you find a security problem, don't open a public issue;
message a maintainer on Discord.

## Hardware testing

Much of what Quadra does depends on the GPU and the compositor. The Metal on Vulkan bridge in particular has
only been qualified on an RTX 3060 Ti; other GPUs are untested until someone runs them. A report that says
"works on my RX 6800, Wayland, 20 minutes of play" is useful. Say what you ran, on what hardware, and what you
did not try.

## Translations

Interface strings live in [`launcher/quadra/i18n.py`](launcher/quadra/i18n.py). English is the source
language: the English text is the key, and `RU` and `ZH_CN` map it to Russian and Simplified Chinese. A string
missing from a table shows in English, so a partial translation is fine. Wrap any new interface text in `_()`.
[#40](https://github.com/QuadraHQ/Quadra/pull/40) added Simplified Chinese and is the model to follow for a
new language.

## Where things are

| Path | What |
|---|---|
| `launcher/quadra/` | The launcher and everything both runtimes share: settings, the window, Roblox install and update, diagnostics, Studio, VR settings, mods |
| `launcher/quadra/darling/` | Launcher code that only the Darling runtime uses: runtime selection and installation, client patches, the web sign-in bridge, stall watching |
| `launcher/quadra/rosemary/` | Launcher code that only the Rosemary runtime uses: installing Rosemary and its Roblox client, starting a session, VR |
| `darling/` | Everything native that only Darling needs ([README](darling/README.md)): `shim/` is injected into Roblox, `frameworks/` holds stub frameworks, `metal-vulkan/` is Metal on Vulkan |
| `rosemary/` | Everything native that only Rosemary needs ([README](rosemary/README.md)): `vr/` builds the VR library Rosemary injects |
| `vr/` | The OpenXR VR device, compiled into both the Darling shim and Rosemary's VR library ([README](vr/README.md)) |
| `studio_layer/` | Roblox Studio's Vulkan layer under Wine (neither runtime) |
| `packaging/`, `flatpak/`, `flake.nix` | AUR, Debian, RPM, Nix and Flatpak recipes |
| `tests/` | Native and Python tests |
| `docs/` | Design notes, runtime notes, bug triage records |
| `install.sh`, `build_debug_shim.sh`, `run_debug.sh`, `launcher/install.sh` | The source installer, the shim build and debug run, and the launcher installer. They stay at the top because packaging and the installer look for them there |

The Darling runtime is a separate repository, [QuadraHQ/CiderDarling](https://github.com/QuadraHQ/CiderDarling),
and the Metal translator is [QuadraHQ/QuadraM2VK](https://github.com/QuadraHQ/QuadraM2VK). Fixes that belong
there should go there.

## Building

You need an x86_64 Linux machine with clang, lld and a Darling runtime to build the shim. Running the launcher
from source also needs Python 3, GTK 4, libadwaita and WebKitGTK 6. The [packaging README](packaging/README.md)
lists the full requirements and versions.

```bash
./build_debug_shim.sh                      # the Darling shim; set DARLING_SYSROOT to pick a sysroot
darling/metal-vulkan/build.sh              # the Metal bridge; needs Rust 1.98 or newer
```

`./run_debug.sh` starts the client from the checkout for manual debugging. When Quadra runs from a source
checkout, the checkout is also its data folder (logs, build output, a copy of the client), so keep that
folder out of anything you commit. Those paths are already in `.gitignore`.

## Testing

```bash
bash tests/run.sh
```

This builds the native fixtures in a temporary directory and then runs the Python tests. It needs clang,
pkg-config and Python 3. The Wayland and OpenXR fixtures run when their development packages are installed.

Tests that need real Darling are separate: `tests/run_darling_*.sh` and `tests/run_darling_*.py`. They are
written to use a temporary prefix, not your live one.

Please test without touching what you actually play on. Don't install over your live Quadra, don't point a test
at your real Darling prefix, and don't use your saved Roblox sign-in. Use a temporary prefix, a copy of the
client, or a throwaway `HOME` and `XDG_*` directories.

A change to the shim or the launcher should come with a test where one can be written. `tests/` has fixtures
for most of the existing areas to copy from. Not everything can be tested without a headset, a particular GPU or
a live game; in that case say so.

## Sending a change

1. Fork the repository and branch from the latest `main`. Maintainers also edit files on GitHub, so pull
   before you start and rebase before you open the pull request.
2. Keep each pull request to one change. Don't mix a fix with a rename or a reformat.
3. Write the commit message as one plain sentence about what changed, in sentence case, with no prefix:
   `Fix the VR check for Rosemary release bundles`.
4. For anything a player could notice, add an entry under **Unreleased** in [CHANGELOG.md](CHANGELOG.md), in
   the style of the entries already there: what was wrong, what changed, and what was checked.
5. In the pull request, say what you ran: unit tests, real Darling, in the game, and on which hardware. Mark
   what you didn't verify. "Build-checked" and "played for an hour" are different claims, and both are fine
   as long as they are accurate.
6. Run `bash tests/run.sh` first. Pull requests that touch the launcher, shim sources, the Metal bridge or the
   Nix files also run the Nix package workflow.

The people who write and review Quadra use generative AI (see the README). Whoever sends a change is still
responsible for it: understand it, run it, and be able to say what it was tested on.

## Things to leave alone

- **Versions and releases.** Maintainers choose version numbers and cut releases. Don't change
  `packaging/release.json`, the AUR recipes' pinned sources, or release entries in the Flatpak metainfo. The
  release workflow runs on `v*` tags and a bot pins the source and AUR recipe afterwards.
- **Compatibility names.** `MACOBLOX_METAL_*`, `CIDER_*`, the `ciderhq-launcher` and `macoblox-launcher`
  entry points and the `Library/CiderHQ` and `Library/MacOBlox` migration paths are kept on purpose, because
  the Darling runtime and the Metal bridge read them or older installs still use them. Don't rename them
  without talking to a maintainer first.
- **Roblox's files.** Don't commit the Roblox client, its shaders or anything extracted from it. Quadra
  downloads the client on first setup.
- **Logs and personal data.** Launch logs, diagnostics archives, screenshots, cookies, join links, tokens and
  webhook URLs never go in a commit. Neither do tester reports or `game_cache.json`.
