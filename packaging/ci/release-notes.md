Quadra 0.40.p1 is a small follow-up to 0.40.

- **Runtime badge:** Home says "Rosemary running" or "Rosemary stopped" when Rosemary is the runtime (it said Darling regardless), and the Roblox version and runtime cards follow the runtime too.
- **Settings action:** the installer's menu entry now has a right-click **Settings** action, which opens the settings window of a running Quadra or starts one. Reinstall to get it on an existing shortcut; the distro packages already include it.
- Everything in the 0.40 notes still applies: direct start with a splash, the new sidebar, Servers, Rosemary as an optional runtime, the mod crash guard and Fast flag allowlist warnings.

Validation: the badge was checked headless for both runtimes ("Darling stopped", "Rosemary stopped"), and the 41 launcher unit tests pass. The Settings action was checked for syntax and on one machine's menu entry, not by clicking it in a panel.

Fully close and reopen Quadra after updating. Studio is not in the Flatpak. OpenGL remains the default for the game; VR requires OpenGL or Vulkan/Zink. Native packages include the matching Metal host and framework. Install `.pkg.tar.zst` on Arch derivatives, `.deb` on Ubuntu 24.04+/Debian 13+, or the binary `.rpm` on current Fedora. Darling/CiderDarling downloads remain separate. Source archives, build recipes and `SHA256SUMS` are attached.
