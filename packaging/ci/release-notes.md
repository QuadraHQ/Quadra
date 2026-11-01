Quadra 0.41.p2 fixes VR being refused with Quadra's own Rosemary. Nothing else changes from 0.41.p1.

- **The bug:** the check that decides whether a Rosemary can load VR read `bin/rosemary`, which in a release bundle (an unpacked AppImage) only launches the real runtime in `libexec/rosemary`. So Quadra's Rosemary, installed through Quadra, was reported as unable to load VR and VR stayed off. The check now reads the real runtime too.
- **Quadra's Rosemary is published:** `0.5.0-quadra.1` (QuadraHQ/rosemary-fork). Settings → Rosemary → "Rosemary build" → "Quadra's Rosemary (adds VR)" installs it; if you picked it while it had no release, you got the regular Rosemary: pick it again, or press Update.
- Everything in the 0.41.p1 notes still applies.

Validation: 90 launcher unit tests. Installing Quadra's Rosemary from its release through Quadra was run in an isolated folder (checksum checked, version `0.5.0-quadra.1`, VR check True; upstream's 0.5.0 is False). The end-to-end VR test passes all six checks with the released AppImage against a headless OpenXR emulator. Not run: a Quest 2 session with the released build.

Fully close and reopen Quadra after updating. Studio is not in the Flatpak. OpenGL remains the default for the game; VR requires OpenGL or Vulkan/Zink. Native packages include the matching Metal host and framework. Install `.pkg.tar.zst` on Arch derivatives, `.deb` on Ubuntu 24.04+/Debian 13+, or the binary `.rpm` on current Fedora. Darling/CiderDarling downloads remain separate. Source archives, build recipes and `SHA256SUMS` are attached.
