Quadra 0.41.p1 adds a choice of Rosemary build. Nothing else changes from 0.41.

- **Rosemary build (Settings → Rosemary):** pick **Quadra's Rosemary (adds VR)** or **Regular Rosemary**. Quadra installs and runs the one you pick, and picking one that is not installed installs it. The two live in separate folders; the tab names the build and version in use, and its VR row says when the current build cannot load VR. The default is Quadra's.
- **Quadra's build** is QuadraHQ/rosemary-fork (release `0.5.0-quadra.1`). If it has no published release when you pick it, Quadra installs the regular Rosemary instead and says so; pick it again once the release is out.
- Everything in the 0.41 notes still applies.

Validation: 88 launcher unit tests pass, including the source choice, the folder suffix, runner kinds and ordering; the Rosemary tab was rendered headless with both builds present. Not run: installing from the fork's release, which was still being built when this was released.

Fully close and reopen Quadra after updating. Studio is not in the Flatpak. OpenGL remains the default for the game; VR requires OpenGL or Vulkan/Zink. Native packages include the matching Metal host and framework. Install `.pkg.tar.zst` on Arch derivatives, `.deb` on Ubuntu 24.04+/Debian 13+, or the binary `.rpm` on current Fedora. Darling/CiderDarling downloads remain separate. Source archives, build recipes and `SHA256SUMS` are attached.
