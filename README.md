<p align="center">
  <img src="data/icons/org.quadrahq.Quadra.png" alt="Quadra logo" width="160">
</p>

<h1 align="center">
 Quadra, by QuadraHQ.
</h1>

 <p align="center">
    <a href="https://discord.gg/W8jXfngc8z">
      <img src="https://img.shields.io/badge/Discord-join-5865F2?logo=discord&logoColor=white" alt="Discord" width="140">
   </a>
 </p>

<p align="center">
  A bootstrapper for the macOS Roblox Player & Studio client, running on Linux through <a href="https://github.com/microlift/rosemary">Rosemary</a>.
</p>

## Install

```
curl -fsSL https://quadrahq.org/install.sh | bash
```

This installs the newest Quadra as an AppImage in `~/.local/bin` and adds it to the app menu. Run it again to update.
Options go after `bash -s --`, for example `curl -fsSL https://quadrahq.org/install.sh | bash -s -- --flatpak`:

- `--flatpak`: install the Flatpak bundle instead
- `--native`: build Quadra on this computer instead
- `--tarball`: same as native, but with pre-compiled binaries
- `--version TAG`: install a specific release, such as `v0.50.0-alpha.3`
- `--uninstall`: remove Quadra but keep its data and your Roblox sign-in; add `--purge` to delete those too

The installer checks every download against the release's `SHA256SUMS` before installing it.

## Build instructions

**Prerequisites:**

- Git
- Rust 1.88+
- GTK4, `gtk4-layer-shell` and libadwaita
- clang for VR support (optional; without it Quadra builds without VR)
- flatpak-builder for building Flatpaks
- curl for building AppImages

**Fetch source:**

```
git clone https://github.com/QuadraHQ/quadra-rewrite && cd quadra-rewrite
```

**Build directly from source:**

```
cargo build
```

**Build Flatpak:**

```
cd flatpak && flatpak-builder --force-clean --user --install-deps-from=flathub --repo=repo --install build-dir org.quadrahq.Quadra.yml
```

**Build AppImage:**

```
./appimage/build.sh
```

The AppImage is written to `appimage/Quadra-x86_64.AppImage`.

**Nix:**

Try it without installing:

```sh
nix run github:QuadraHQ/quadra-rewrite
```

Flake input:

```Nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    quadra-rewrite.url = "github:QuadraHQ/quadra-rewrite";
  };
}
```

```Nix
{
  environment.systemPackages = with pkgs; [
    inputs.quadra-rewrite.packages.${pkgs.stdenv.hostPlatform.system}.quadra-rewrite
  ];
}
```

## Relevant directories

- `~/.local/share/quadra`: Where Quadra stores its data
- `~/.local/share/quadra/prefix`: The Rosemary prefix Roblox runs in
- `~/.local/share/quadra/rosemary/<version>`: Installed Rosemary versions
- `~/.local/share/quadra/roblox`: The Roblox Player installation
- `~/.local/share/quadra/logs`: Quadra and Roblox logs
- `~/.local/share/quadra/modifications`: Your own files that Quadra overlays onto the Roblox client (see the Mods settings)
- `~/.local/share/quadra/mods-backup`: The original Roblox files that mods replaced, so they can be put back
- `~/.cache/quadra/downloads`: Downloaded files, such as Rosemary and Roblox packages
- `~/.cache/quadra/vr`: The VR library Rosemary loads into Roblox
- `~/.local/share/butter/runners` and `~/.var/app/io.github.microlift.butter/data/butter/runners`: Butter runner directories that Quadra also detects runners from

## License

GPL-3.0-only.
