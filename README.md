<p align="center">
  <img src="data/icons/hicolor/256x256/apps/org.quadrahq.Quadra.png" alt="Quadra logo" width="160">
</p>

<h1 align="center">
 Quadra, iced edition
</h1>

<p align="center">
  A bootstrapper for the macOS Roblox Player & Studio client, running on Linux through <a href="https://github.com/microlift/rosemary">Rosemary</a>.
</p>

This is redybcs's fork of [QuadraHQ/Quadra](https://github.com/QuadraHQ/Quadra). It keeps up with upstream, and its [`iced-gui`](https://github.com/redybcs/Quadra/tree/iced-gui) branch replaces the GTK4/libadwaita interface with [iced](https://iced.rs):

- Starts in about half the time and allocates about half the memory of the GTK version
- Its own title bar, which follows your desktop's button layout (or switch back to the system one)
- Seven themes, including Aurora, Ember, Paper and Phosphor, with GPU-drawn animated backgrounds that pause when Quadra isn't in use
- Seven background animations to use with any theme, in your own colours
- Your own themes, imported from CSS files (see [Custom themes](#custom-themes))

For help with Quadra itself, upstream has a [Discord server](https://discord.gg/W8jXfngc8z).

## Install

This fork doesn't publish releases yet, so build it from source (below) or run it with Nix. The installer at quadrahq.org installs upstream's GTK version, not this one.

## Build instructions

**Prerequisites:**

- Git
- Rust 1.92+
- A C compiler (Quadra's GUI, iced, loads Wayland/X11, xkbcommon and the GPU driver at runtime, so no GUI headers are needed)
- clang for VR support (optional; without it Quadra builds without VR)
- flatpak-builder for building Flatpaks
- curl for building AppImages

**Fetch source:**

```
git clone --branch iced-gui https://github.com/redybcs/Quadra && cd Quadra
```

**Build directly from source:**

```
cargo build --release
```

The program is written to `target/release/quadra`.

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
nix run github:redybcs/Quadra/iced-gui
```

Flake input:

```Nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    quadra.url = "github:redybcs/Quadra/iced-gui";
  };
}
```

```Nix
{
  environment.systemPackages = with pkgs; [
    inputs.quadra.packages.${pkgs.stdenv.hostPlatform.system}.quadra
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
- `~/.config/quadra/themes`: Themes you imported from CSS files
- `~/.local/share/butter/runners` and `~/.var/app/io.github.microlift.butter/data/butter/runners`: Butter runner directories that Quadra also detects runners from

## Custom themes

Settings > General > App theme has a + button that imports a theme from a CSS file. Quadra only reads custom properties, so a theme sets colours and a few other values, not selectors or layout:

```css
:root {
  --name: "Midnight";
  --window: #0d1520;
  --card: rgba(255, 255, 255, 0.05);
  --text: #e6edf3;
  --accent: #58a6ff;
  --radius: 12px;
  --font: "Inter";
  --background: linear-gradient(180deg, #0d1520, #10263d);
  --animation: waves;
  --animation-colors: #1d4f91, #2389c9, #7fd3f0;
}
```

- Colours: `--window`, `--card`, `--text`, `--dimmed`, `--accent`, `--on-accent`, `--accent-text`, `--border`, `--strong-border`, `--raised`, `--hovered`, `--pressed`, `--field`, `--dialog`, `--shade`, `--title-bar`, `--destructive`, `--success`, `--warning`. Use `#rgb`, `#rrggbb`, `#rrggbbaa`, `rgb()` or `rgba()`.
- `--radius` in pixels, `--font` and `--mono-font` as the name of an installed font, and `--name` for the name shown in Settings (the file name otherwise).
- `--background`: a colour or a `linear-gradient(…)` with up to 8 colours. It shows when the theme has no animation.
- `--animation`: `aurora`, `embers`, `leaves`, `scanlines`, `waves`, `fireflies`, `bokeh` or `none`, with up to three `--animation-colors`.

Anything left out comes from Adwaita's light or dark look, picked by how bright `--window` is. Values Quadra can't read are skipped and named in the log. The animation and its colours can also be changed for any theme under Background animation.

## License

GPL-3.0-only.
