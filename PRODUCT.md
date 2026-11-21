# Product

<!-- impeccable:product-schema 1 -->

## Platform

desktop

A native Linux desktop app (Rust, iced 0.14 on wgpu, with a software renderer fallback). Not a web app and not a mobile app.

## Users

Linux players who want to launch and play Roblox, first. Roblox Studio developers on Linux, second. Both use the same app: a Roblox/Studio switch on the Home page picks which one Quadra launches.

## Product Purpose

Quadra is a bootstrapper that installs, updates and runs the macOS Roblox Player and Studio clients on Linux through Rosemary, with the settings, mods and integrations around them (FastFlags, sounds, cursors, fonts, VR, app menu entries, link handling, server joining and history).

This repository is redybcs's fork of QuadraHQ/Quadra. Its `iced-gui` branch replaces upstream's GTK4/libadwaita interface with iced. It will not be merged upstream, but it keeps up with upstream. Success means a player opens Quadra, gets into Roblox quickly, and enjoys an interface that feels faster and more personal than the GTK version without losing anything it can do.

## Positioning

Three things together, which upstream's GTK version does not offer:

- **Speed and lightness:** starts in about half the time and allocates about half the memory of the GTK version, and stays cheap on old laptops.
- **Deep theming:** built-in themes, GPU-drawn animated backgrounds usable with any theme in the user's own colours, a theme gallery with live previews, a cursor glow, and themes imported from CSS files.
- **Same features, better feel:** feature parity with upstream; the fork changes how it feels, not what it does.

## Operating Context

- Used on Linux desktops of every kind (GNOME, KDE, tiling window managers), on X11 and Wayland, from modern GPUs down to machines with only a software renderer (llvmpipe).
- Opened briefly to launch a game, then left in the background or closed while Roblox runs; settings are visited occasionally.
- Window sizes range from about 360×420 up to maximised; the title bar follows the desktop's window button layout or can be swapped for the system one.
- Distributed by building from source or with Nix; Flatpak and AppImage build recipes exist. The fork publishes no releases yet. Upstream's installer at quadrahq.org installs the GTK version, not this one.

## Capabilities and Constraints

- **Upstream compatibility:** settings, wording and features stay close to upstream so regular merges stay easy. New fork-only features should sit alongside upstream's, not reshape them.
- **Software rendering must work:** every screen, theme and animation must work without a GPU, and stay cheap there. Animated scenes fall back to still backdrops on the software renderer; anything animated asks for frames only while it changes and settles when idle or unfocused.
- **Motion:** an Animations setting (Full, Reduced, Off) governs all motion and is lowered to Reduced when the desktop asks for less motion.
- **Themes:** custom themes are CSS files of custom properties in `~/.config/quadra/themes`; new theme properties must be documented in the README.
- **Code conventions:** self-explanatory code with no comments; views are cheap and side-effect free (see AGENTS.md).
- **Commits:** authored by the maintainer only, never authored or co-authored by an AI, per AI_USAGE_POLICY.md.
- **Terminology:** "Roblox" and "Roblox Studio" for the clients; "Rosemary" for the runner; "mods" for the client modifications on the Mods tab; "FastFlags" for Roblox's flags.

## Brand Commitments

- Name: Quadra. The README calls this fork "Quadra, iced edition".
- Logo: the Quadra app icon at `data/icons/hicolor/` (a rotated, layered square on a purple-to-pink gradient).
- Upstream community: QuadraHQ's Discord server and quadrahq.org belong to upstream; the fork points to them for help with Quadra itself.

## Evidence on Hand

- Measured performance claims in the README (start time and memory versus the GTK version).
- No testimonials, user counts, reviews or releases exist for the fork; do not invent any.

## Product Principles

1. Getting into the game comes first: launching Roblox or Studio is never more than one obvious action away.
2. Personal, but never at the cost of speed: every visual flourish has to earn its frame budget and work on the software renderer.
3. Parity before novelty: the fork never drops or reshapes something upstream can do; it adds alongside.
4. Stay mergeable: keep upstream's settings, wording and structure recognisable.
5. Respect the desktop: follow its button layout, colour scheme and motion preferences by default.
