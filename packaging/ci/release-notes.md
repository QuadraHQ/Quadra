Quadra 0.43.2 lets the launcher start on distributions without GLibUnix, which 0.43.1 meant to fix but did not fully. Nothing else changes from 0.43.1.

- **Launcher signals:** 0.43.1 went back to `GLib.unix_signal_add`, but the UI still required the `GLibUnix` typelib on load, so the launcher kept failing to start where it is missing. GLibUnix is now optional: the launcher uses GLibUnix's signal function with GLib 2.80 and newer and `GLib.unix_signal_add` otherwise.
- **Flatpak:** the README lists the Flatpak as experimental rather than discontinued, and points to the bundle attached to each release.
- Everything in the 0.43.1 and 0.43.0 notes still applies: VR on Roblox 0.745 and 0.746, the shared launcher state, Discord Rich Presence for Flatpak and Snap clients, the GNOME 51 Flatpak and the Nix Rosemary variant.

Validation: the 470 unit tests pass and the metainfo validates with appstreamcli. The launcher's signal setup was checked on GLib 2.84, once with GLibUnix and once with GLibUnix made unavailable, and SIGHUP reached its handler both times. The native tests were not run for this release (no clang on the release machine), and the launcher was not started on a distribution that lacks GLibUnix.

Fully close and reopen Quadra after updating. Studio is not in the Flatpak. OpenGL remains the default for the game on Darling; VR requires OpenGL or Vulkan/Zink. Native packages include the matching Metal host and framework. Install `.pkg.tar.zst` on Arch derivatives, `.deb` on Ubuntu 24.04+/Debian 13+, the binary `.rpm` on current Fedora, or the Flatpak with `flatpak install --user quadrahq-0.43.2-x86_64.flatpak` (it uses Flathub's GNOME 51 runtime). Darling/CiderDarling downloads remain separate. Source archives, build recipes and `SHA256SUMS` are attached.
