Quadra 0.42.p1 brings a new logo, makes GNOME the default launcher appearance and keeps the logo in colour in the Monochrome appearance. Nothing else changes from 0.42.

- **New logo:** the nested tilted squares replace the old logo in the application and tray icons, the setup and Play pages, the README, the website and the Discord icon. The old Tux square and the `macoblox-*` icons are gone from the tree.
- **GNOME is the default:** a new install opens in the system appearance instead of Grid. An install that already saved an appearance keeps it, and Settings → Environment → Interface → Launcher appearance still switches between GNOME, Grid and Monochrome.
- **Monochrome keeps the logo's colours:** every other image there is still drawn in grey.
- **VR warning:** worded more briefly, in English, Russian and Chinese.
- Everything in the 0.42 notes still applies.

Validation: 453 launcher unit tests and the nine GTK fixtures pass headless (Xvfb, Cairo renderer); the appearance fixture reads real pixels to check that the logo is coloured and the rest grey in Monochrome, and fails when that is broken. Not run: the launcher on a real desktop with the GL or Vulkan renderer, and no package, Flatpak or Nix build. The website and Flatpak screenshots still show the old Cider launcher.

Fully close and reopen Quadra after updating. Studio is not in the Flatpak. OpenGL remains the default for the game on Darling; VR requires OpenGL or Vulkan/Zink. Native packages include the matching Metal host and framework. Install `.pkg.tar.zst` on Arch derivatives, `.deb` on Ubuntu 24.04+/Debian 13+, or the binary `.rpm` on current Fedora. Darling/CiderDarling downloads remain separate. Source archives, build recipes and `SHA256SUMS` are attached.
