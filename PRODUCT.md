# Product

<!-- impeccable:product-schema 1 -->

## Platform

desktop

A native Linux desktop app (Rust, iced 0.14 on wgpu, with a software renderer fallback). Not a web app and not a mobile app.

## Users

Linux players who want to launch and play Roblox, first. Roblox Studio developers on Linux, second. Both use the same app: a Roblox/Studio switch on the Home page picks which one Quadra launches.

## Product Purpose

Quadra is a bootstrapper that installs, updates and runs the macOS Roblox Player and Studio clients on Linux through Rosemary, with the settings, mods and integrations around them (FastFlags, sounds, cursors, fonts, VR, app menu entries, link handling, server joining and history).

Quadra's interface is built with iced, replacing the earlier GTK4/libadwaita one. Success means a player opens Quadra, gets into Roblox quickly, and enjoys an interface that feels fast and personal without losing anything the GTK version could do.

## Positioning

Three things together, which the earlier GTK version did not offer:

- **Speed and lightness:** starts in about half the time and allocates about half the memory of the GTK version, and stays cheap on old laptops.
- **Deep theming:** built-in themes, GPU-drawn animated backgrounds usable with any theme in the user's own colours, a theme gallery with live previews, a cursor glow, and themes imported from CSS files.
- **Same features, better feel:** feature parity with the GTK version; the interface changes how Quadra feels, not what it does.

## Operating Context

- Used on Linux desktops of every kind (GNOME, KDE, tiling window managers), on X11 and Wayland, from modern GPUs down to machines with only a software renderer (llvmpipe).
- Opened briefly to launch a game, then left in the background or closed while Roblox runs; settings are visited occasionally.
- Window sizes range from about 360×420 up to maximised; the title bar follows the desktop's window button layout or can be swapped for the system one.
- Distributed through the installer at quadrahq.org (AppImage, Flatpak or a native build), from source, or with Nix.

## Capabilities and Constraints

- **Compatibility:** settings, wording and saved configuration stay compatible with earlier releases. New features sit alongside existing ones rather than reshaping them.
- **Software rendering must work:** every screen, theme and animation must work without a GPU, and stay cheap there. Animated scenes fall back to still backdrops on the software renderer; anything animated asks for frames only while it changes and settles when idle or unfocused.
- **Motion:** an Animations setting (Full, Reduced, Off) governs all motion and is lowered to Reduced when the desktop asks for less motion.
- **Themes:** custom themes are CSS files of custom properties in `~/.config/quadra/themes`; new theme properties must be documented in the README.
- **Code conventions:** self-explanatory code with no comments; views are cheap and side-effect free (see AGENTS.md).
- **Commits:** authored by the maintainer only, never authored or co-authored by an AI, per AI_USAGE_POLICY.md.
- **Terminology:** "Roblox" and "Roblox Studio" for the clients; "Rosemary" for the runner; "mods" for the client modifications on the Mods tab; "FastFlags" for Roblox's flags.

## Brand Commitments

- Name: Quadra, by QuadraHQ.
- Logo: the Quadra app icon at `data/icons/hicolor/` (a rotated, layered square on a purple-to-pink gradient).
- Community: QuadraHQ's Discord server and quadrahq.org.

## Evidence on Hand

- Measured start time and memory against the GTK version (about half of each).
- No testimonials, user counts or reviews are on hand; do not invent any.

## Product Principles

1. Getting into the game comes first: launching Roblox or Studio is never more than one obvious action away.
2. Personal, but never at the cost of speed: every visual flourish has to earn its frame budget and work on the software renderer.
3. Parity before novelty: never drop or reshape something Quadra can already do; add alongside it.
4. Stay familiar: keep settings, wording and structure recognisable between releases.
5. Respect the desktop: follow its button layout, colour scheme and motion preferences by default.
