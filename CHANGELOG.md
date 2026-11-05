# Changelog

## Unreleased


## 0.43.2 — 2026-11-05

- The launcher starts again on distributions whose GLib bindings do not provide `GLibUnix`. 0.43.1 still asked for
  `GLibUnix` when loading the UI, which failed there. Quadra now uses GLibUnix's signal function when it is
  available (GLib 2.80 and newer) and `GLib.unix_signal_add` otherwise.
- The README lists the Flatpak as experimental instead of discontinued.

## 0.43.1 — 2026-11-04

- Fix VR on Roblox 0.746, which said "Unavailable: unsupported client graphics device or VR emulator". Roblox added
  two methods to its graphics device, which moved the VR device's getVR and setVR two slots down; Quadra now finds
  them by their code, so 0.745 and 0.746 both work.
- The launcher handles signals through `GLib.unix_signal_add` again instead of `GLibUnix`, which some distributions'
  GLib bindings do not provide.

## 0.43.0 — 2026-11-04

- The launcher's state is one observable store shared by the pages (`core/state.py`). Settings, the Play page, the
  tray, the logs view and the Studio page read and watch the same values, so a change made in one place shows
  everywhere at once instead of after a restart. The Play page shows the renderer in use: with Rosemary it follows
  the Force OpenGL switch instead of always saying OpenGL.
- Discord Rich Presence:
  - It finds the sockets of Discord clients in a Flatpak or a Snap (the Discord and Discord Canary Flatpaks, Vesktop,
    and any other Flatpak app's socket), not only a natively installed Discord.
  - It uses Roblox's Discord application.
  - Game details are looked up again every minute while a game is played, and a lookup that fails part of the way keeps
    what an earlier one found, so renamed games update in the saved game cache.
  - Placeholder names (`[TITLE UNAVAILABLE]`, `[UNKNOWN]`) are never shown or saved, and an icon is only used once
    Roblox says it is ready.
  - A game saved without an icon is looked up again straight away, so the icon appears when the Show game icon setting
    is on.
  - Text is fitted to Discord's 2 to 128 character limits.
  - Problems are printed in the launcher's output with Discord's reason, once each (no Discord found, a rejected
    update).
- Deprecated GTK APIs are replaced. The file pickers (Rosemary program, death sound, cursor, font, fast-flags export)
  use `GtkFileDialog`, and a picker that fails shows an error instead of nothing. The dark style follows the system
  through the CSS provider's colour scheme instead of `gtk-application-prefer-dark-theme`. Stylesheets load with
  `load_from_string` and signals are handled through `GLibUnix`. The launcher needs GTK 4.12 or newer.
- Flatpak:
  - The recipe uses the GNOME 51 runtime, which supplies GTK 4, libadwaita, PyGObject and WebKitGTK on Freedesktop
    26.08, instead of building them, so a build no longer compiles WebKitGTK.
  - The launcher draws with OpenGL (set `GSK_RENDERER` to override it).
  - It can reach the Discord, Discord Canary and Vesktop Flatpaks' sockets.
  - The 1024×1024 icon is no longer exported, which Flatpak refused.
  - CI builds it, and releases attach `quadrahq-<version>-x86_64.flatpak`.
- Nix: a Rosemary variant of the package and module (the `rosemary` flake outputs). With Nix managing Rosemary, the
  runtime choice in Settings and the onboarding page is fixed to it. The CiderDarling Metal host hash is fixed, and CI
  checks the Rosemary runtime.
- The release workflow runs on its own for a pushed commit whose message starts with `Release` and that changes
  `__version__`. It also checks that `__version__` and `packaging/release.json` agree, and builds from the pushed
  commit itself.
- Development: `.envrc` loads the Nix development shell through direnv, the uv setup is removed again, the README
  describes Quadra as a bootstrapper for the macOS Roblox Player and Studio, and the launcher is formatted with ruff.

## 0.42.p3 — 2026-11-02

- Rosemary: Settings → Roblox has a **Clear Roblox cache** button and a switch that clears the cache before each
  launch. The client keeps the interface updates Roblox ships over the air in the prefix (`OTAPatchBackups`, next to
  the `rbx-storage` asset cache): a fresh prefix runs the interface bundled with the client, and every later launch
  runs the cached update. On the machine where this was found, a fresh prefix worked once and each launch after it
  failed (Roblox's VR bottom bar never appeared, the 3D world stayed black in VR, and the logs show `SetCore:
  ResetButtonCallback has not been registered by the CoreScripts`) until the prefix was cleared. Clearing deletes
  `OTAPatchBackups` and `rbx-storage` and nothing else, so the sign-in stays and the next launch runs the bundled
  interface again. That launch downloads the updates again, so the switch has to stay on to keep it that way. The
  button is refused while a game runs. The switch is off by default and makes games load their assets again on every
  launch. No Fast flag does this: Roblox only applies the flags on its public allowlist, and the client's patch flags
  (`DataModelPatcherForceLocal` and the rest) are not on it.
- 0.42.p2 was tagged but its release workflow failed and nothing was published. Its Clear Roblox cache removed
  `rbx-storage` but not `OTAPatchBackups`, so the cached interface survived, and the workflow's regression job lacked
  the PyGObject and libadwaita packages that `installer_runtime_test` needs; the job now installs them.
- The launcher's settings follow the runtime. Darling-only settings and the Quadra M2VK
  tab are hidden while Rosemary is selected, the Rosemary Roblox group moved into the Roblox tab, and the Rosemary
  runner selector is clearer.
  The runtime and the theme are chosen in the onboarding page, and the bootstrapper no longer needs Darling or lld to
  run. `app.py` and `core.py` are split into the `core/` and `ui/` packages, and the repository has a Nix development
  flake.
- Checked: the launcher's 459 Python tests pass with a clean configuration directory (two of them read the real
  `~/.config/quadrahq/fastflags.json` and fail if it holds flags), and eight of the nine GTK fixtures pass headless
  (Xvfb, Cairo renderer). `tests/m2vk_settings_ui.py` fails with `('Fast', [])` on `main` before this release and
  still does. Not run: the new button and switch in the launcher on a real desktop, a package, Flatpak or Nix build,
  and whether clearing only the cache (not the whole prefix) removes the black VR world.
- 0.42.p1 was tagged but never published; its changes are part of this release:
- New logo (nested tilted squares, purple to pink) everywhere the old ones were: the application and tray icons at
  every size, the setup and Play pages, the README, the website and its favicon, the Discord application icon image and
  the wordmarks (`branding/make_wordmark.py` rebuilt the PNG wordmarks, which had still read "Mac O' Blox"). The old
  Tux square and its sources (`branding/square.svg`, `branding/source/Tux.*`), the `macoblox-*` icons and the README's
  Tux credit are removed. The icons are cut from the designer's PNG exports, not from the SVG: librsvg draws the SVG's
  noise and shadow filters wrongly, so the scalable icon is still not installed. The screenshots on the website and in
  the Flatpak metadata still show the old Cider launcher.
- GNOME (the system appearance) is now the launcher's default appearance, where it was Grid. An install that already
  saved an appearance keeps it; a settings file that never saved one now opens in GNOME.
- In the Monochrome appearance the logo on the Play page and in setup keeps its colours; everything else is still drawn
  in grey.
- The VR warning is worded more briefly (it no longer compares VR to a Roblox cheat); the Russian and Chinese
  texts follow.
- Checked: the launcher's 453 Python tests and the nine GTK fixtures pass headless (Xvfb, Cairo renderer), including a
  pixel check that the logo is coloured and the rest grey in Monochrome, and the new test fails when that is broken. Not
  run: the launcher on a real desktop with the GL or Vulkan renderer, and no package was built.

## 0.42 — 2026-11-01

- The source tree is sorted by runtime. Darling's native code moved to `darling/` (`shim/` for the injected shim and its
  helpers, `frameworks/` for the stub frameworks, `metal-vulkan/` for Quadra M2VK, which was `prototype/metal-vulkan/`),
  Rosemary's VR build to `rosemary/vr/`, and the OpenXR device both runtimes compile to `vr/`. In the launcher, the
  modules only Darling uses are in `quadra/darling/` and the ones only Rosemary uses in `quadra/rosemary/`. Nothing
  changes for players. `install.sh`, `build_debug_shim.sh` and `run_debug.sh` stay at the top. The shim built from the new
  layout is the same as the old build except for the linker's UUID; the Nix package, Flatpak, the release workflow and
  the distro recipes were updated by reading and have not been built. The first launch after updating rebuilds the shim.
  The first Metal-to-Vulkan experiment (`experimental/metal/`, offscreen only) and the AIR upgrade prototype script are
  removed.
- Rosemary is now the default runtime for a new install, and Darling is marked outdated. `install.sh` asks which to use
  (`--backend rosemary` or `--backend darling`; `--darling ...` still means Darling). Choosing Rosemary installs only the
  launcher and its tools: no Darling, no shim build, and the launcher downloads Rosemary and Roblox on the first Play.
  Updates keep the saved choice, and an install whose settings were saved before the choice existed stays on Darling,
  as do the Flatpak and Nix packages, which carry Darling. `--uninstall --purge` now also removes Rosemary's files
  (`~/.local/share/quadrahq`). Checked with stubbed installs and the launcher's tests; a real install with Rosemary
  has not been run from the installer.
- Turning on VR now asks first. VR works by changing the running client's memory, which is also what a Roblox cheat
  does, so Roblox's anti-cheat could flag it; the chance is thought to be low but is not known. The warning has a
  "Don't show this again" switch, kept only when you go ahead. Installs with VR already on are not asked.
- `QUADRA_REFRESH_RATE=<hz>` makes the client see that refresh rate (`-[NSScreen maximumFramesPerSecond]` and the
  refresh intervals, which Darling's NSScreen does not define), for testing high-refresh and variable-refresh displays.
  Unset, nothing changes. Checked in Darling that the screen reports it; not checked what the client does with it.
- `MACOBLOX_METAL_PRESENT_MODE=fifo_relaxed` selects relaxed FIFO where the surface offers it, plain FIFO otherwise.
- `tools/check_imports.py` lists the symbols a Roblox client imports that Darling, the shim and the injected frameworks
  do not export. On 0.744 it finds 38, among them `CVPixelBufferGetDataSize`, `CVPixelBufferRetain` and
  `SMAppService`; static, so it does not say which are ever called.
- Controllers now work. Darling's GameController framework is stubs and its IOHID has no Linux devices, so
  `[GCController controllers]` could never return a pad. The shim now reads the kernel's evdev gamepads itself (raw
  Linux system calls, since Darling does not translate evdev ioctls) and implements `GCController` with the extended
  gamepad profile: sticks, triggers, d-pad, face and shoulder buttons, menu/options/home, stick clicks, connect and
  disconnect notifications for hotplug, value and pause handlers, and `physicalInputProfile`. Xbox, DualSense and Switch
  Pro layouts follow the Linux gamepad convention. Steam Input's virtual "X-Box 360 pad" (a Steam Deck in desktop mode,
  with Quadra added to Steam as a non-Steam game) is recognised and uses the Xbox X/Y layout. No rumble, motion, light or battery yet. `QUADRA_GAMEPAD=0` turns it
  off. The `GCInput*` button keys the client looks controls up by (`GCInputButtonA` and the rest) are defined now;
  Darling's framework has none, so the lookups returned nil. Tested against a virtual pad on `/dev/uinput` in real Darling and natively; not yet tried with real controllers
  or in the Roblox client.
- Metal on an integrated AMD GPU no longer fills the GPU's GTT (the system RAM it may map) until the driver fails a
  submission and Vulkan loses the device (#38). The 0.35 and 0.36 budget caps only changed the figure the game is told;
  a 0.37.p4 log on a Renoir laptop with a 135 MiB budget still grew live graphics memory from 0.5 to 3.2 GiB in twelve
  seconds, nearly all of it outside device-local memory, and was lost. The host now reads the adapter's own GTT counters and refuses a host-visible buffer of 1 MiB or
  more that would leave less than a sixteenth of the GTT (at least 128 MiB) free, after waiting up to 250 ms for work in
  flight to give memory back. The game receives plain memory in its place and a draw that uses it sees no buffer. The
  recommended working set also follows the GTT that is still free. Other GPUs are not checked, and
  `MACOBLOX_METAL_GTT_GUARD=0` turns the check off. Not yet confirmed on the affected hardware.
- Fix the segmentation fault after a lost Vulkan device. The game was handed `nil` buffers and wrote through their NULL
  `contents`. The device cannot be recovered, so the host now logs the graphics memory at that moment, writes the
  performance trace's last sample and closes the game with status 87.
- The hardware report records, for AMD adapters, the driver's VRAM, visible VRAM and GTT totals and use, with the
  `amdgpu.gttsize` and `ttm.pages_limit` settings, so a report shows which ceiling an integrated GPU had.
- The Metal performance trace records the GTT in use, how many buffers and textures the game made, released and holds,
  what the new check refused or delayed, and whether the device was lost. Memory that keeps growing can now be traced
  to the game or to the runtime.

## 0.41.p2 — 2026-11-01

- Fix VR being refused with Quadra's own Rosemary installed from its release: the check read `bin/rosemary`, only the
  launcher in a release bundle, instead of the real runtime in `libexec/rosemary`. It now reads both.
- Quadra's Rosemary 0.5.0-quadra.1 (QuadraHQ/rosemary-fork) is published. Nothing else changes from 0.41.p1.

## 0.41.p1 — 2026-11-01

- Settings → Rosemary has a "Rosemary build" selector: Quadra's Rosemary (QuadraHQ/rosemary-fork, adds what VR needs)
  or the regular Rosemary (microlift). Quadra installs the one picked, keeps them in separate runner folders, runs
  the chosen one (the other still runs if it is all that is installed) and names the build in use. Quadra's build
  falls back to the regular one only while it has no release, and says so. Default: Quadra's.
- Nothing else changes from 0.41.

## 0.41 — 2026-11-01

- The old launcher layout is back: the sidebar lists Play, Settings, Studio, Servers, Mods and Info, the Play page is
  the old dashboard (its badge names the runtime, Darling or Rosemary) and Settings is one Environment page with a
  tab bar and a Rosemary tab. 0.40's grouped sidebar and Home page are gone; the direct start, tray icon and Servers
  page stay, and the new settings live in the old pages.
- Rosemary is an optional runtime (Settings → Rosemary). Quadra installs Rosemary from its release (checksum checked)
  and Roblox for it when you press Play or pick it; "Install Roblox" did nothing with Rosemary selected, which is
  fixed. It gets mods (backups per build), the CPU limit set before it starts, Discord status, rows for its options,
  "Hide the macOS menu bar", going back to the previous build, update-required relaunch and sign-out.
- Settings only one runtime reads are greyed out and tagged "Darling only" (backends.py).
- VR on Rosemary with Quadra's build of Rosemary (QuadraHQ/rosemary-fork, quadra-vr): `libQuadraVR.dylib` (vr_device.c
  and vr_xr.c, built on first use by rosemary_vr/build_rosemary_vr.sh) is injected into Roblox
  (`ROSEMARY_DYLD_INSERT_FORCE`) and calls the host's OpenXR through `quadra_host_*`. Roblox runs on OpenGL for it;
  an option runs that on Vulkan through zink. `QUADRA_ROSEMARY` picks the Rosemary program for one run.
- Fast flags are kept in Quadra's config (`fastflags.json`), apart from the flags Darling adds (`core.DARLING_FLAGS`),
  which only go into the Darling client's file.
- Tests: 82 in `tests/launcher`; `rosemary_vr/test_rosemary_vr.sh` runs the VR path against the headless emulator
  (`~/.local/share/quadrahq/vr-emulator`).

## 0.40.p1 — 2026-11-01

- Home says "Rosemary running" or "Rosemary stopped" when Rosemary is the runtime (it always said Darling), and its
  Roblox version and runtime cards follow the runtime too. Translated into Russian and Chinese.
- The menu entry the installer writes has a right-click Settings action (), as the distro
  packages' entry already did.
- Nothing else changes from 0.40.

## 0.40 — 2026-11-01

- Quadra starts Roblox straight away. `quadrahq` shows a small splash (logo, progress, Cancel), updates Roblox when a
  newer version is out, and starts the game; the settings window opens from the tray icon, `quadrahq --settings` (also
  in a running Quadra) or the shortcut's right-click Settings action (reinstall for an existing shortcut). It still
  opens by itself for first-time setup, a missing client and errors. "Open settings after Roblox exits" replaces "Show
  the launcher after Roblox exits", and Quadra now closes with the game unless it is on.
- New sidebar: Home (Roblox/Studio choice, status cards, recent games), Servers and Mods; a ROBLOX group (Roblox, Game,
  Fast flags, Quadra M2VK, Rosemary, VR, CiderDarling); Configuration (Appearance, Integrations, Network, Environment,
  Diagnostics). The Environment page was split into those pages; no setting was removed.
- Servers page: history of joined servers with Rejoin, optional join by region through RoValra's server list (RoValra
  sees the game id and the region), join by id or roblox.com link. A desktop notification names the server's location
  when "Show server location" is on. Rejoin and region joining hand Roblox a `roblox://experiences/start` link and are
  not yet tried in a game.
- Rosemary (microlift/rosemary) can run Roblox instead of Darling: Settings → Rosemary installs Rosemary from its
  release (checksum checked) and the macOS client, which stay apart from the Darling client. Adapted from nutmeg
  (GPL-3.0). Tried in a live game on one machine. No mods, Discord status or web sign-in window with it; MangoHud is
  switched on through `MANGOHUD=1`, because wrapping Rosemary in the `mangohud` program crashed it.
- Fast flags Roblox's own allowlist (`DFStringAllowedPublicFlags`) does not include get a warning icon.
- Mod crash guard: when Roblox crashes within five minutes of a start with new top-level entries in `modifications/`,
  they are renamed `.disabled` (listed on the Mods page, one click to re-enable). Nothing is set aside without an
  earlier good session to compare with.
- "Block Roblox telemetry" answers Roblox's telemetry and crash-upload hosts with NXDOMAIN through the DNS forwarder
  (needs a DNS server other than the system's). A toast says when the game you joined was updated in the last day.
  Experimental CPU core limit. A dedicated VR icon.
- Tray icon (StatusNotifierItem with a menu: Open settings, Stop Roblox, Quit); checked against a fake panel, not a
  real one. `launcher/quadra` has 41 unit tests in `tests/launcher`.

## 0.37.p4 — 2026-10-26

- Fix VR controllers never vibrating. Roblox only passes vibration to the controllers while the headset reports a
  focused session that has controllers able to vibrate, and Quadra reported neither. `VRService.VRSessionState` now
  follows the headset too (Idle, Visible, Focused, Stopping) instead of staying Idle.
- Nothing else changes from 0.37.p3.

## 0.37.p3 — 2026-10-25

- Fix the middle mouse button and the extra mouse buttons doing nothing in game. Darling's window code has no case for
  them and dropped every press, drag and release, on X11 and on native Wayland. They now reach Roblox the way the left
  and right buttons do.
- Fix keys outside the main block. On X11, Insert, Right Alt/AltGr, Menu, F13 to F20, keypad = and the < > | key beside
  Left Shift reached Roblox as the A key, because Darling's key table stops at an Apple keyboard. They now arrive as the
  keys a Mac sends for them (Insert as Help). On native Wayland the same keys were dropped and are now sent.
- Nothing else changes from 0.37.p2.

## 0.37.p2 — 2026-10-25

- Fix text that vanished in the Grid look (and in Monochrome). A long subtitle could squeeze the chosen value of a
  dropdown down to an ellipsis: Settings -> Quadra M2VK -> Accuracy showed "…" instead of "Fast". Every dropdown now
  keeps its value whole.
- Dim disabled buttons and switches once, not twice. Newer libadwaita (1.10 checked) already fades them with a filter,
  and the theme faded them by opacity as well, which left them at about a fifth of their strength. "Apply and restart Quadra"
  was nearly unreadable while it was disabled. A disabled main button now has a readable style of its own.
- Nothing else changes from 0.37.p1.

## 0.37.p1 — 2026-10-25

- Build Studio's Vulkan layer into the Arch, Debian and Fedora packages. The 0.37 packages did not contain it: the build
  containers had no Vulkan headers, so the layer was skipped and Settings -> Studio -> Graphics -> Vulkan said the fix was
  not built. The recipes now require the headers (`libvulkan-dev`, `pkgconfig(vulkan)`, `vulkan-headers`) and the package
  build fails when the layer is not in the package. The layer compiles in the Ubuntu 24.04, Fedora and Arch images. The
  Nix package still builds without it. Nothing else changes from 0.37.

## 0.37 — 2026-10-25

- Add a Simplified Chinese translation (zh_CN), contributed by 2112992430 in pull request #40. It covers 446 of the 498
  strings the launcher translates, including the Studio section; the rest stay English until it is completed.
- Run Roblox Studio on Kombucha, the Wine build Vinegar patches for Studio, instead of Kron4ek's staging build. Studio
  now follows the newest stable Kombucha release as well as Roblox's newest Studio: each start checks both, and an
  existing install moves to Kombucha in place (prefix, sign-in and the installed Studio are kept; DXVK is installed
  again for the new build). Without a connection the installed Wine is used.
- Give Roblox Studio a section of its own in the launcher (Studio, Graphics, Wine and updates, Fast flags):
  - Install Microsoft's WebView2 runtime in Studio's prefix (version 144.0.3719.92, checked against the SHA-256 Microsoft
    publishes). Without it Studio stopped at "Launch Failed" when it tried to show its login; now it shows the login,
    Toolbox and Creator Hub pages itself.
  - Choose the renderer: DXVK (as before), DXVK-Sarek 1.11.0 for older graphics cards, Wine's Direct3D through OpenGL
    (also at feature level 10), or Vulkan with Vinegar's layer for the swapchain errors Studio does not handle (built
    with the shim when the Vulkan headers are installed). DXVK's DLLs now sit next to Studio instead of in the prefix's
    `system32`, so the renderer can change without recreating the prefix; an install from before has them moved.
  - Choose the graphics card Studio draws on, and ask GameMode to optimise it while it runs.
  - Show the place you are building and whether you are in a script as Discord presence, read from Studio's log.
  - Keep Studio's sign-in and settings in `studio/backup` (mode 0600) and restore them into a new prefix; give a new
    prefix Studio's Local AppData, Documents and Pictures in your own folders; follow the system light/dark theme.
  - Stay on a Kombucha release or use a Wine of your own, stay on one Studio version or channel (the signed-in
    account's own channel by default, found with the saved Roblox login), and edit Studio's fast flags.
  - Copy the files in `~/.config/quadrahq/studio-overlay` over Studio's at every start.
  - Fetch Studio's packages from Roblox's alternate hosts when `setup.rbxcdn.com` refuses or fails.
  - Start Studio through a `quadrahq --studio` process that stays until Studio closes (GameMode, presence and the
    backup need one); a second start, such as the browser handing over a sign-in, only launches.
- Show Quadra's current logo in Discord presence: the application's own `ciderhq` image is still the old apple, so the
  logo is now served from quadrahq.org.
- Finish the MacOBlox/Cider to Quadra rename inside the code: the launcher's Python package is `quadra`, the shim is
  `libQuadraShims.dylib` (native Wayland helper `libquadra-wayland.so`), and the settings variables are `QUADRA_*`.
  `MACOBLOX_*` variables set by hand are still read as `QUADRA_*`. `MACOBLOX_METAL*` (read by Quadra M2VK) and the
  `CIDER_*` names and CiderDarling identifiers that the Darling runtime reads keep their names until those projects
  are renamed.
- Move to the `QuadraHQ/Quadra` repository. The installer and the updater move an existing checkout's `origin` from
  `CiderHQ-org/Cider`, and the installer still recognises checkouts that have `launcher/macoblox`. Runtime update
  feeds that name `CiderHQ-org/CiderDarling` are accepted next to `QuadraHQ/CiderDarling`.

## 0.36.p1 — 2026-10-24

- Fix the camera continuing to turn after the right mouse button was released, when the left button had been clicked
  during the right-button drag. The game never saw the right button go up (or the drags after the left release), so it
  kept the mouse lock until the menu was opened. Darling's `NSWindow` keeps one mouse-down location for every button
  and invalidates it whenever any button goes up, which left nothing to deliver the other button's drag and release
  to. The window event hook now keeps the location while the other button is still held. This applies to X11 and
  native Wayland, including the releases sent when the Wayland window loses focus.
- Add an AppKit regression to `tests/darling_input_test.m` that sends right-down, left click and right-up (and the
  reverse order) to a recording view. Before the change the view received `3 1 2` and never the right release.

## 0.36 — 2026-10-24

- Stop the menu freezing for about 18 s after leaving a game. Roblox's `RBX IoEvLoop` threads wait up to 10 s per
  `kevent()` and were not woken for the work that follows a leave; they now look at their queues every 100 ms
  (setting `kevent_poll_ms`, default 100, 0 turns it off). The game ran at full speed 2 s after a leave in 3 of 3 trials
  instead of 18 s in 9 of 9. Only threads with that name are affected. It is a workaround for Darling's kqueue
  emulation: what those loops were not woken for is still not identified.
- Update Quadra M2VK: the pinned runtime enables `VK_EXT_memory_priority` and pageable device-local memory by default
  (an 8 GiB card made 9,216 MiB of device buffers where it failed at 7,168 MiB before), and the performance trace
  splits live memory into device-local and host.
- Add a Quadra M2VK settings tab (accuracy, present mode, memory priority, compile threads, warm states, VRAM limit and
  fallback). Before launch the GPU path is proven with a self-test, and Quadra falls back to OpenGL once when the GPU
  cannot run Metal, with a clearer message when no driver is found.
- Log the GPU and driver identity and the runtime's account of an unmeasured driver, say why a drawable acquisition
  timed out and wait only 100 ms for an image when nothing of ours can free one, and name large or slow buffer
  creations (16 MiB or more, or 50 ms or more).
- Prefer the libLLVM a process already maps and find Debian's `libLLVM.so.1` by its directory, so the legacy shader
  upgrade works with LLVM 18, 19 and 23.
- Add the GPU and driver, the session type, desktop, Xwayland version, window backend, raw mouse setting and the
  `[Quadra Input]` lines to `quadrahq --hardware-report`.
- Add `quadrahq --watch-stalls`, which records the game's per-thread CPU use, stacks of busy threads, what blocked
  threads wait on (futex words, timeouts, sockets, pipes, timers) and reports them against the launch log's join and
  leave markers. The game must be started with the freeze diagnostics setting on. It never stops the game's threads
  more than 60 times a second.
- Host the website and report form on the VPS, with a detailed diagnostics
  collection guide, archive-content checks for both diagnostics switches,
  private report storage and queued Discord delivery with attachments.
- Deploy the website from each main-branch push using a signed webhook and
  periodic recovery sync. Keep private services outside the repository.

## 0.35.p2 — 2026-10-23

- Add a website bug-report guide linked to a public GitHub form requiring a
  diagnostics `.tar.gz`, reproduction steps and the affected configuration.
- Keep the public Roblox 403 fallback moving past an alternate origin's
  missing upload, rate limit or temporary server failure. Update the deployed
  mirror to retry official archive origins on a cold cache.

## 0.35.p1 — 2026-10-23

- Stop launch immediately when Darling reports unavailable OverlayFS support.
  Show module-loading, kernel-reboot and custom-kernel recovery instructions
  in the startup error dialog, retaining the original error in the launch log.
- Keep Roblox setup and routine update checks on the public release channel,
  even with a saved restricted rollout channel or an old required-update log.
  Preserve explicit recovery of the deployment requested by a running client.
- Fix native Wayland clipboard crashes when a rich-text view pastes its local
  RTF copy. Transfer supported plain text, preserve Unicode across legacy and
  UTF-8 string APIs, and keep private pasteboard formats local.
- Cap low-memory integrated-GPU Metal budgets at 512 MiB, scale larger APUs
  with RAM, and refresh with host/cgroup memory pressure during play. Limit
  the estimated footprint of optional resident shader preloads to a quarter
  of that graphics budget. The reported Renoir device-loss case still needs
  validation on the affected hardware.
- Add NixOS packaging with a bundled CiderDarling runtime and NixOS module.
- Remove the launcher's top tagline and welcome subtitle. Replace the ready
  badge with the selected Darling runtime's running/stopped status, refreshed
  while the Play page is visible.

## 0.35 — 2026-10-22

- Redesign the launcher with square controls, a grid backdrop and a compact
  dashboard. Add immediate, saved appearance choices for Grid, the original
  GNOME interface, and aubree.lat's dark monochrome style. Keep running
  sessions, logs and open settings intact when switching.
- Retry Roblox deployment 403s on alternate official metadata and archive
  origins before the Quadra mirror. Cold mirror downloads use those archive
  alternatives too, retaining channel/version validation and atomic installs
  (#36).
- Keep Shift Lock's requested cursor position in event order on native
  Wayland. Cancel interrupted camera-drag capture when switching windows on
  Wayland and X11, while allowing button-free Shift Lock to resume (#37).
- Bound Metal on Vulkan's graphics budget by the selected device's heap,
  Vulkan's available memory budget where supported, and system RAM headroom
  on integrated GPUs. Use a conservative 512 MiB fallback (#38).

## 0.34 — 2026-10-22

- Integrate the pinned Quadra M2VK performance and stability fork, including
  native Wayland presentation, pipeline retention and resource lifetime fixes.
- Prepare learned pipelines, installed shader libraries and bounded local
  resources before playing, with progress and Skip and play controls. Add
  optional performance diagnostics and memory-aware compilation limits.
- Add verified RakNet compatibility for Roblox **0.744.0.7441080**
  (`version-546c6da637854de3`). Inspect the relocated transport selection,
  fallback and shared/legacy dummy-connection paths, and recognize the new
  startup-throttle location. Retain complete executable SHA-256 validation
  and atomic replacement.
- Recognize the official 0.744 GLSL pack and repair the same four whitespace
  sites in its unchanged HeightmapDebugPS sources. Preserve all shader tokens,
  descriptors and source ranges; complete pack and source hashes are required.

## 0.32 — 2026-10-19

- Resume the experimental Metal on Vulkan renderer: replace Darling's Metal
  framework, present through a native Wayland subsurface, and skip Metal
  layers in Darling's bitmap compositor to prevent the startup crash.
- Ship the prebuilt Metal bridge in Arch, Debian/Ubuntu and Fedora packages
  and add the renderer to Settings. Verify Roblox login, home
  and Write a Letter on NVIDIA, plus exact render/readback and MSAA resolve.
  OpenGL remains the default; VR still requires OpenGL or Zink.

## 0.31 — 2026-10-19

- Retry public Roblox version and macOS client downloads through Quadra's
  self-hosted cache when the original endpoint returns HTTP 403 (#36). Keep
  account traffic on Roblox and reject incomplete client downloads.
- Preserve right-drag camera motion when left-click is pressed during mouse
  capture; release either button without leaving a frozen pointer (#35).
- Record one automatic FPS-drop marker per sustained 45+ to 20-or-lower FPS
  transition. The intermittent 60-to-15 FPS issue (#34) still needs a trace
  captured during a drop; this diagnostic does not change game pacing.

- Rebrand Cider / CiderHQ to **Quadra / QuadraHQ** at **quadrahq.org**, keeping
  the existing logo. The command is `quadrahq` and the app ID is
  `org.quadrahq.Quadra`.
- Migrate Cider and MacOBlox settings, cache, source installs and Darling
  session files without replacing existing Quadra data. Keep CiderDarling's
  runtime identifiers and the current GitHub repositories compatible.
- Release native packages under `quadrahq` and update Flatpak packaging. The
  legacy Cider 0.30 AUR recipe remains pinned; the Quadra release source and
  stable AUR recipe are published with checksums.

## 0.30 — 2026-10-17

- **Experimental VR.** Play in Roblox's own VR mode on a headset connected
  through the host's OpenXR runtime: WiVRn for Meta Quest, Monado for PC
  headsets. Turn it on in the new **Settings → VR** tab, start the runtime with
  the headset connected, then play. The macOS client still contains Roblox's
  VR mode; Cider supplies the OpenXR device the Mac build leaves out. The VR
  home menu, games, head tracking, Touch/Index controllers, vibration and
  recentering work. OpenXR paces each frame, and each image is submitted with
  the head pose Roblox drew it from, so tracking stays smooth. Render
  resolution (70% of the headset's recommendation by default) and a
  side-by-side mirror in the game window are adjustable; the headset keeps the
  refresh rate chosen in the runtime. Requires the OpenXR loader. Tested on a
  Quest 2 with WiVRn 26.9; details are in [docs/VR.md](docs/VR.md).

- Fix duplicated typed characters and an unreliable Control key with native
  Wayland. Chat and search no longer insert a key twice; Ctrl shortcuts
  (select all, copy, cut, paste, word navigation and deletion) work, and focus
  changes release held modifiers.

- The website shows VR gameplay and the VR settings tab.

The brief freeze after leaving a game remains unresolved.

## 0.25 — 2026-10-15

- Apply native Wayland fullscreen requests made before the platform window
  exists. Previously Cocoa retained the fullscreen flag while the native
  window stayed decorated and resizable. A real AppKit regression reproduces
  that mismatch under private KWin and checks initial fullscreen plus repeated
  fullscreen/windowed toggles with the repair.

- Warn once per launch when Roblox actually selects a game transport other
  than RakNet. Explain that the Roblox update broke game compatibility,
  games might fail or show Error 256, and users should update Cider or wait
  for a compatibility patch. Offer log and update-check actions, show the
  warning even with the launcher hidden or Discord presence disabled, and
  keep unsupported-version headers alone from triggering a false alarm.

- Support Roblox **0.743.0.7431134** (`version-212bac6bdccd4673`) in the
  verified RakNet compatibility patch. This hotfix relocates the transport
  and dummy-connection code, so the previous build's patch was correctly
  rejected and Error 256 returned. Verify the new executable against the
  official archive and retain complete normalized SHA-256 validation,
  atomic replacement and compatibility with the startup-throttle patch.

The brief freeze after leaving a game remains unresolved and is deferred to a
later update.

## 0.24p1 — 2026-10-13

- Add verified RakNet compatibility for Roblox **0.743.0.7431129**
  (`version-f641ec9ebf6145c3`). The new executable was unsupported and selected
  RbxTransport despite the saved flags, causing Error 256 under Darling.
  Inspect the relocated transport/fallback code and skip both shared-config
  and legacy dummy connections introduced by the new caller layout. Retain
  complete normalized SHA-256 checks, atomic replacement and compatibility
  with the relocated startup-throttle patch. The reporter confirms game
  joining works after applying the fix to Roblox 0.743.
- Keep a visible software cursor at the right-click camera drag position on
  Wayland and X11/Xwayland. Preserve the last visible cursor image when Roblox
  hides it or selects a transparent cursor, with an arrow fallback. Camera
  movement continues normally; releasing right-click or losing focus clears
  the override. Pressing right-click during existing Shift Lock or first-person
  capture keeps that capture's cursor policy.

## 0.24 — 2026-10-13

- Repair native Wayland startup and game-join freezes caused by unsupported
  Darling IOKit HID discovery and Apple platform service queries. SDL continues
  to provide keyboard and mouse input.
- Align Wayland mouse clicks with the visible cursor using the window height
  and a consistent virtual Cocoa window origin. Keep warp/capture requests
  independent of AppKit calls on rendering threads.
- Resize the parent Wayland background and commit window geometry when the
  compositor changes the client size. The outer border now follows the game
  image; game views keep rendering on independent EGL subsurfaces.
- Rename the launcher backend option to **Wayland** and remove the experimental
  wording from its settings, errors and launch logs.
- Add regressions for IOKit query ownership/delegation, pointer conversions,
  and actual parent-buffer commits during fullscreen and restore. Native
  OpenGL and Vulkan/Zink checks pass on the local NVIDIA/niri setup; Roblox
  joining, aligned clicks and border resizing are confirmed on that machine.

## 0.23patch2 — 2026-10-13

- Repair recursive `__sincos*_stret` math wrappers in old Darling builds from
  Cider's shim, including the 4312 CiderDarling build in the AMD startup reports.
  Volatile sine/cosine calls prevent Clang from recreating the recursion.
- Keep monitoring a live selected-prefix Roblox process when `darling shell`
  disconnects with status 1. Previously the launcher treated this as a game
  failure and terminated that still-running guest (issue #27).
- Remember the previous CiderDarling installation and offer **Use previous
  CiderDarling and restart** in Settings. Retain the current prefix and sign-in;
  reject missing or damaged installations and block switching during a game.
- Resume interrupted runtime downloads and reuse checksum-verified downloads.
  A failed archive download keeps the selection and does not start a long
  source build automatically; `--from-source` remains available.
- Add **Collect hardware report** and `ciderhq --hardware-report --graphics`.
  Reports separate GPU/MSAA/CPU-limit/lifecycle probes from user-reported
  gameplay observations; diagnostics include basic hardware inventory.
- Keep Roblox's crash handler available when **Allow freeze diagnostics** is
  enabled. Add **Trace text input** for numeric insertion/selection diagnostics.
- Require launcher regressions as well as all distro package builds before
  publishing Cider packages. Remaining Zink crashes, duplicate typing and KDE
  Wayland sizing/closing reports still require affected-machine confirmation.

- The launcher's Website link shows a bundled globe icon. It used the
  icon theme's `web-browser-symbolic`, which KDE's Breeze does not have, so
  Breeze users saw a missing-icon symbol.

## 0.23patch1 — 2026-10-12

- Cider is now licensed under the GNU GPL v3 (`LICENSE`), as the README
  states. Narezy's original Mac O' Blox MIT notice is kept in
  `LICENSES/MIT-MacOBlox.txt` and installed with the packages; the Arch,
  Debian, RPM and Flatpak metadata declare `GPL-3.0-only AND MIT`.
- **Install in terminal** no longer fails with "Unable to find terminal
  required for application" when `xdg-terminal-exec` is missing: Cider finds
  the terminal itself (`$TERMINAL`, the desktop's own terminal,
  `x-terminal-emulator`, kitty, Alacritty, foot, WezTerm, Ghostty, Konsole,
  GNOME Terminal and others) before falling back to GIO, whose list older
  GLib versions keep short. With no terminal at all, the error says what to
  install or set.
- Discord Rich Presence uses CiderHQ's own Discord application, so it reads
  "Playing CiderHQ" with the Cider logo (the old "Mac O` Blox" application had
  no uploaded art, so its `macoblox` image key showed nothing). It also shows
  the Cider version on hover, a **Get Cider** button and, when the experience
  is shown, a **Game page** button.
- Rich Presence clears again 2 and 10 seconds after a game ends, on the same
  connection. Legcord and Vesktop (arRPC) apply an update only after looking
  up its images through Discord's API but apply a clear at once, so an update
  still resolving when the game closed could land after the clear and stay.

## 0.23 — 2026-10-12

- Discover complete CiderDarling CI releases in the background and offer
  **Update CiderDarling** automatically. Checks run at startup and hourly,
  can be disabled in Settings, and preserve a cached release through outages.
  Updates install the exact advertised revision in a separate terminal;
  newer/custom runtimes are not downgraded.
- Build and publish Arch, Ubuntu/Debian and Fedora packages in CI. Releases
  publish only after every package succeeds, with a verified immutable source
  archive and automatically refreshed stable AUR recipe.
- Let native package users select their runtime in the launcher without
  requiring the regular Darling package alongside a CiderDarling installation.

- Fix joining a game freezing in older Darling's `__exp10`/`__exp10f`:
  use a compiler-safe power call in the shim, including when a copied math
  library hides the runtime's library. Existing runtimes can use the repair
  after rebuilding the Cider shim.
- Diagnostics resolve overlay library paths against the selected runtime
  and preserve full Objective-C method names in stack symbols.

- Install CiderDarling from its CI's prebuilt build when one suits the
  system (Arch-based, or Debian/Ubuntu-based with glibc 2.39+): download,
  check the pinned SHA-256, install its libraries and unpack setuid root, in
  minutes instead of an hour of compiling. `--from-source` (or
  `CIDER_DARLING_FROM_SOURCE=1`) compiles as before, which also remains the
  fallback. `packaging/pin-ciderdarling.py <revision>` pins a revision with
  its archive checksums.

- Prefer the CiderDarling revision pinned in `install.sh` over a remembered
  older one, offer **Update CiderDarling** in Settings, and save the updated
  installation (same prefix) from both the launcher and
  `install.sh --install-runtime`. Detect an interrupted pinned installation
  and offer to remove it (`--replace-incomplete-runtime` without a terminal).
  Delete build trees after a successful build and earlier leftovers before
  a new one; keep the build log, and show its end when an install fails.
- Add **Settings → CiderDarling** (GPU, renderer, watchdog kicks, runtime
  trace, verbose darlingserver log, diagnostics, prefix folder), `ciderhq --ciderdarling-settings`, and a *CiderDarling Settings*
  menu entry while CiderDarling is installed.
- Add **Collect diagnostics** (menu and `ciderhq --collect-diagnostics`): an
  archive with every game thread's wait sampled twice, logs (join tickets and
  cookies redacted) and the runtime selection. With **Allow freeze
  diagnostics** (`MACOBLOX_ALLOW_DEBUGGER=1`) the game lets it read each
  thread's instruction pointer and stack, as library + offset and symbol
  (Darling's own libraries included); `--gdb` adds gdb's backtraces, registers
  and stack tops, with the same names.
- Shim: watchdog thread kicks are off unless `MACOBLOX_KICK=1`. Each kick ran
  Darling's signal handler (several darlingserver requests at an arbitrary
  point), and Darling restarts untimed waits after a signal anyway, so a
  stalled UDP reader stayed stalled for up to 10 s. Stalls are still logged.
  A condition wait with a practically infinite timeout no longer overflows
  into an immediate timeout (a busy loop).
- Pin CiderDarling a8dae5f8e: the `__sincosf_stret` startup crash fix, thread
  requests that are never refused (a queue starved for good once 64 GCD
  workers were busy), and fewer darlingserver round trips per I/O call.
- Server location lookups are now opt-in (they send the server's IP address
  to RoValra), run off the interface thread, read only the end of the
  selected prefix's Roblox log, cache failures and stop with the game.
- Start darlingserver with its own output file instead of the launch log,
  ignore inherited `DSERVER_LOG_*` (`CIDER_KEEP_DSERVER_LOG=1` keeps them),
  cap launch logs at 200 MB in total and rotate `dserver.log` past 32 MB.
- Re-stage the repaired libSystem/libsystem_c and ffmpeg bridge copies in
  the prefix when the runtime's own files change, and report prefix
  libraries Cider did not make instead of touching them.
- Give Roblox 6 s to save its settings after it starts quitting; remove audio
  FIFOs left by a killed launcher and clean up on SIGTERM/SIGINT/SIGHUP;
  ignore browser join links older than 10 minutes; clear `last-error.txt`
  after a successful launch or install.

## 0.22Patch1 — 2026-10-12

- Add an installer choice between regular Darling and pinned CiderDarling,
  plus launcher controls to switch between installed runtimes.
- Add server location detection to the game logs view.
- Move installer/update URLs, project links and package sources to
  `CiderHQ-org/Cider`. Migrate recognized legacy Git remotes on update and
  refresh the release archive root and SHA256 after the repository rename.
- Prepare the pinned Darling source for Clang/LLVM 23: disable supported
  Objective-C constant literal families, replace the reported empty literals
  with Foundation factory calls, and guard the split branch opcodes for older
  LLVM. Add Vulkan headers/loader to Arch source-build dependencies.
  These changes follow the CachyOS field report; a full Darling rebuild and
  Metal rendering have not been verified here. The compile prompt already
  reads `/dev/tty` for piped installs.
- Make the MangoHud switch control activation despite inherited environment
  variables or a previous Darling session. Record the requested state and
  recognize the reported `/proc/<pid>/fdinfo` permission crash, offering to
  disable the overlay before another launch. The switch saves immediately;
  the reported disagreement with settings.json has not been reproduced.
- Apply the supplied CiderHQ logo to the website, README, launcher welcome,
  window icons and installed shortcuts. Preserve its colors and transparency
  and provide PNG sizes for desktop integration.
- Publish the installer as a static website file so Vercel does not need a
  build script outside its selected deployment directory. Refresh it with
  `bash website/build.sh` whenever the source installer changes.

## 0.22 — 2026-10-11

- Rebrand to **Cider**, `ciderhq` native packages/command and `org.ciderhq.Cider`
  desktop identity. Move settings, caches, source installs and guest session
  data from legacy paths without replacing existing Cider data.
- Serve the source installer at `https://ciderhq.org/install.sh`; update the
  launcher, setup guide, desktop entries, website and colored vector branding.
- Correct frozen mouse-lock positions for X11 multi-monitor desktop heights.
  Match Roblox's first-screen coordinate conversion and reject off-window
  targets before updating the logical cursor or overlay.
- Restore public Darling builds that skip optional Swift LFS downloads and
  never request GitHub credentials; retain live build output.
- Add complete checksum/layout-verified GLSL compatibility for Roblox
  **0.742.0.7421053** alongside 0.741; record shader status/hash in launch logs.
- Retry initially failed native Wayland EGL drawables with bounded backoff;
  log backend, EGL driver, drawable creation and exhausted recovery.
  The reported blank window still needs confirmation on the affected system.
- Clear the legacy GTK dark-theme preference in the launcher process before
  initializing libadwaita; keep styling controlled by AdwStyleManager.
- Rename Arch, Debian/Ubuntu and Fedora/openSUSE package recipes; generate
  stable AUR metadata from the prepared archive. Local pre-release source
  preparation excludes user reports, build output and the website.

# Release notes

## 0.21patch1 — 2026-10-07

- Extend custom Roblox UI scaling to scroll/magnification positions and
  engine-originated mouse position overrides. Keep the AppKit mouse cache
  in physical coordinates during first-person lock; apply the extra UI scale
  once at each engine handoff. Validate the additional client methods and
  instruction layouts before enabling scaling.
- Replace the X11 locked-pointer fallback's guessed warp deltas with native
  positions and X request serials. Batch pointer deltas like raw motion,
  distinguish motion queued before and after recentering, and fall back
  after 100 ms of missing raw input when physical motion was observed.
  Match raw/core server timestamps and ignore motion queued before selection
  so stopping the mouse does not disable a working raw stream.
  Negotiate XI2 per connection and release acquired event cookies.
- Keep the selected AppKit cursor bitmap for the X11 locked-cursor overlay,
  avoiding snapshots of a different or invisible cursor under the confined
  physical pointer. Preserve image dimensions and bitmap hot spots.
- Add checksum-verified transport compatibility patches for Roblox
  0.742.0.7421053, including its relocated transport selection, fallback and
  dummy-connection branches. Accept the existing startup-throttle patch when
  verifying the executable. Write the transport patch status into each launch
  log, including the executable size and hash for unsupported clients.
  These changes build; gameplay on the reporting systems still needs
  confirmation. The reported whole-game freezes and disconnections have not
  been reproduced, and full logs are still needed to identify their causes.
- Decode native Wayland custom cursors directly from their source bitmap,
  sharing the X11 decoder instead of drawing through Darling's image
  compositing path. Keep logical cursor size and hot spots when resampling
  larger representations, and clear invalid transparent color channels.
  Supply straight alpha to SDL and premultiplied alpha to locked-cursor
  Wayland surfaces, avoiding a second multiplication at translucent edges.
  The native shim and helper build; the reporting user's appearance still
  needs confirmation.
- Move the experimental Flatpak recipe to Freedesktop 26.08, bundling the
  GTK 4, libadwaita, Adwaita icons, Python bindings and WebKitGTK UI stack.
  Disable Vulkan rendering in those UI libraries while retaining Roblox's
  renderer choices.
  Match graphics and Vulkan layer extensions to the new runtime branch.
  Include the startup-patch helper missing from the previous Flatpak payload.
  Normalize WebKit bindings into the standard introspection lookup directory
  and apply cleanup across the dependency payload. The full experimental
  Freedesktop build succeeds locally; the reported overlay issue still needs
  confirmation.
- Prepare a checksum-pinned 0.21 `macoblox` AUR recipe and update the existing
  `macoblox-git` recipe. Build the shim and Wayland helper during package
  creation, include the startup-patch helper and desktop/MIME integration,
  declare audio and renderer dependencies, and generate `.SRCINFO` files.
  The stable package builds locally; no AUR submission has been made.
- Add Debian/Ubuntu packaging and a Fedora/openSUSE RPM spec using a shared
  payload installer and checksum-pinned release source preparation. Declare
  distro-specific UI and audio dependencies, prebuild the shim and native
  helper, and include desktop/MIME integration. Require a separate full
  Darling package. Debian and RPM source packages build; target-distro binary
  builds remain unverified.

## 0.21 — 2026-10-06

- Fix required-update loops when Roblox requests a different deployment
  channel from the launcher's default. Recover the validated version and
  upload ID from the client's update response, download that official upload,
  and verify the bundle version before replacing the installed client.
  Remember and show the client's update channel, handle gated channel checks
  without sending copied credentials, and avoid offering older deployments.
- Detect whether `pw-cat` supports raw audio. Use `pacat` through PipeWire's
  PulseAudio server on older versions, including PipeWire 1.0.5. Install the
  fallback on Debian/Ubuntu/Mint, retain playback errors in session logs and
  fall back after native playback failures.
- Apply intentional locked-cursor position changes to the visible overlay and
  frozen event coordinates on the event thread. Keep native confinement and
  unlock restoration, coalesce repeated requests, and suppress background
  cursor warps after focus loss. Shift-lock visibility still needs gameplay
  confirmation on the reporting system.
- Prevent public Darling source downloads from prompting for Git credentials.
  Skip the separate optional Swift SDK LFS downloads and exclude their pointer
  placeholders from source installs. The current Roblox client does not link
  that runtime; source-built Darling does not gain Swift SDK support.

### Packaging and verification

This is a source release. No Flatpak bundle is included in 0.21.

The native shim and framework stubs build successfully; changed Python sources
parse successfully and the installer passes shell syntax checks. The official
0.742 deployment requested in the report has the expected bundle version and
no linked Swift libraries. Mint playback, shift-lock icon visibility and a full
Darling source installation have not been verified on the reporting system.

## 0.20 — 2026-10-05

### Input fixes

- Replace the successful empty Carbon key-label translation with a valid ANSI
  fallback layout and bounded printable-key translation. This repairs the
  missing keyboard labels used by Roblox's E-key interaction prompts on both
  rendering backends; normal AppKit text input keeps its existing path.
- Pair custom Roblox UI scaling with absolute mouse-position compensation to
  correct the growing click offset at 200% and other custom scales. Preserve
  raw camera deltas and native cursor coordinates. Unsupported client input
  layouts skip custom scaling and report the reason in the launch log.
- Confine camera capture to the native game window, use absolute recentering,
  and restore the saved pointer position on unlock. Release capture on focus
  loss and window teardown; correct the X11 focus-event window field.
  Apply capture changes on the event thread, acknowledge cursor hiding for
  each lock transition, and recover from cursor-worker connection failures.
- Supply real attributed text substrings for Roblox's composition offsets and
  repair Darling's collapsed-selection anchor in the Roblox text handler.
- Add optional `MACOBLOX_TRACE_TEXT_INPUT=1` diagnostics for typing reports.
  They record lengths, ranges and composition state without typed content.

### Crash diagnostics

- Record session lifecycle state before cleanup, identify packaging and client
  version, and prevent guest log writes from overwriting lifecycle records.
  Preserve Darling warmup output and its exit status in the launch log.
- Expand opt-in fatal signal coverage and remove unsafe stack scans and
  signal-time symbol lookup. Record registers and bounded window-startup
  call markers; rename the setting to **Crash diagnostics**.
- Correct misleading EGL retry messages: report whether a retry occurred,
  its actual error, and the native window ID.

### Stability and responsiveness

- Add **Graphics card** under Game settings when multiple GPUs are detected.
  Store PCI identities, apply Mesa/NVIDIA PRIME selection to both the host
  probe and Darwin game, and use the selected adapter's measured VRAM budget.
  Automatic preserves desktop/terminal selections; unavailable stored adapters
  produce an actionable error instead of silently choosing another GPU. Clear
  stale Darling GPU selectors before restoring each session's selection.
- Recognize Roblox quitting for a required update even when it exits through
  the normal quit sentinel. Offer a fresh official-client download and keep
  pending game links until update recovery completes.
- Forward the host configuration directory to Linux graphics layers, avoiding
  attempts by layers such as LSFG to create settings under Darwin's `/Users`.
- Bound embedded-browser socket work per UI turn and cap queued frames,
  callbacks, pages and injected scripts. Dispose old pages and cancel pending
  work on disconnect; reject stale replies after a new game session connects.
  Release unused guest web views and suppress SIGPIPE on private socket writes.
- Serialize Discord IPC, coalesce presence updates on one worker, and disconnect
  after incomplete replies. Keep one metadata worker per tracker, atomically
  save a bounded game cache, and ignore activity callbacks from ended sessions.
  Start tracking only while Rich Presence is enabled and restart it on enable.
- Include queued DNS work in the upstream deadline and cap pending queries.
  Close active connections on shutdown, bound custom hostname lookups, and honor
  short/zero DNS TTLs with aged cached replies.
- Limit live-log reading and highlighting per UI update, retain fragmented
  UTF-8 lines, and bound displayed text and search matches. Reuse the scroll
  mark and preserve the selected search match during automatic updates.
- Skip unchanged X11 drawable geometry and visibility requests after validating
  the backend ABI and native-window identity. Preserve OpenGL context preparation
  and invalidate cached state after off-main-thread changes.
- Bound private X11 setup/visual queries to 250 ms and pointer snapshots/cursor
  transitions to 100 ms. Retry interrupted I/O, limit reply sizes, suppress
  SIGPIPE, and move the one-time Xauthority lookup off input/render callers.
- Read shader diagnostic sources from the current GL context instead of a
  process-global shader-name cache. Cap diagnostic allocations and avoid extra
  compilation-status waits when shader tracing is disabled.
- Make watchdog location capture signal-safe with atomic request ownership.
  Record only the interrupted instruction pointer without scanning application
  stacks or taking dynamic-loader locks.
- Poll and clean up game sessions outside GTK's main thread. Keep one watcher
  per launch and ignore callbacks from old sessions.
- Roll back partially initialized host audio, make audio shutdown idempotent,
  and reap microphone recorders after a forced shutdown.
- Recognize rootless Flatpak guests and orphans from their loader and mapped
  prefix runtime files, restoring game tracking and scoped cleanup without
  treating the shared sandbox mount namespace as ownership evidence.

### Packaging

This is a source release. No Flatpak bundle is included in 0.20.

### Verification and remaining work

The native shim, Wayland helper and framework stubs build successfully;
changed Python sources parse successfully. This stability/performance pass
has not run gameplay checks or benchmarks, and these changes have not been
verified on the reporting users' hardware. E-key prompt visibility and hybrid
laptop GPU offloading still need gameplay confirmation. Keyboard labels use
an ANSI fallback; selecting among multiple proprietary NVIDIA cards with
OpenGL requires an X11 provider selection.

The ordinary-character deletion report, the two previously reported crashes,
the pause after leaving games and intermittent graphics stutters remain under
investigation. Native Wayland remains experimental; Vulkan still uses Zink.

## 0.19 — 2026-10-04

### Bug fixes and settings

- Fix invisible X11 cursors with Vulkan (Zink), including fast image creation,
  cursor changes and recreated drawable windows.
- Allow independent Darwin DNS lookups to run concurrently instead of waiting
  behind one slow lookup. Retain serialized fallback when resolver isolation
  cannot be confirmed.
- Add **Roblox UI scale** in Game settings, from 100–400%. The `dpi_scale`
  setting now reaches Roblox; 200% enlarges its interface while preserving the
  rendering resolution. Default scaling stays at 100%.
- Stop adding `DFFlagDisableDPIScale` through the texture-quality preset.
  Existing custom flags are preserved; remove that flag to use UI scaling.
- Preserve the Flatpak Mesa driver discovery improvements and show useful
  Vulkan probe failures without inheriting Darling's preload library.

### Experimental Wayland

- Give concurrent AppKit views separate EGL drawables; repair resizing,
  hide/show and drawable lifetime handling.
- Repair recursive window lookup and display initialization cleanup.
- Improve NVIDIA Vulkan discovery without X11, while preserving explicit
  Vulkan driver selections and filters.
- Keep Native Wayland opt-in and experimental; X11 / Xwayland stays the default.

### Verification and remaining work

The native shim and helper built successfully. Sixteen compiled regression
fixtures and 99 Python tests passed. Actual Darling/X11 cursor and concurrent
DNS fixtures passed. The production UI scaling wrapper visibly enlarged the
actual Roblox interface at 200%; scaled click alignment still needs an
end-to-end check. Hardware EGL, shared contexts and Wayland window lifecycle
fixtures passed, but full native Wayland client presentation is not verified.

The pause after leaving a game remains unresolved; a focused client reproduced
an approximately 14-second frame gap. Intermittent Vulkan stutters, direct
client Vulkan and the other user's exact crash remain under investigation.
Vulkan rendering still uses Mesa Zink. Custom Darlingserver multiworker builds
are isolated experiments and are not included in this release.
The Flatpak is a testing package; this release does not claim new Flatpak
gameplay verification.

## 0.18 — 2026-10-04

### Setup

- The terminal installer now welcomes you, explains its destination and system
  packages, shows four installation steps, and gives clear next steps.
- First launch has a welcome, installation overview, download progress, retry
  screen and sign-in guidance. Reopen it from **Setup guide** in the menu.
- Existing installations keep their settings and Roblox session. Source updates
  back up local changes before replacing tracked files and preserve other files.

### Bug fixes

- Fix raw mouse and right-click camera freezes caused by synchronous modifier
  queries. Bound event processing and preserve mouse deltas and button events.
- Restore requested mouse capture after Alt-Tab or another focus change.
- Fix crashes while rendering Private Servers and other embedded pages; improve
  browser bridge timeouts and page closure handling.
- Improve Zink context switching, drawable handling, shader compatibility,
  GPU memory reporting and frame pacing.
- Reduce Darling file-close cleanup costs during the initial leave-game
  transition. Repair event wakeups and lock error handling.
- Keep cleanup within the selected Darling prefix and use stable process
  handles to avoid affecting other app environments.

### Verification and remaining work

The repaired input, focus, Private Servers and initial leave transition were
checked with the actual macOS client on an RTX 3060 Ti using Xwayland. Native
regressions and mocked launcher/process regressions passed. The installer and
setup changes received syntax/build checks and a local visual preview; system
package installation was not repeated for this release.

A later approximately 10–11 second pause after leaving a game is still under
investigation. Intermittent graphics stutters need more gameplay measurements.
Vulkan gameplay still uses Mesa Zink; the Metal-to-Vulkan work is a prototype.
Native Wayland is an opt-in experimental feature and is not complete.
