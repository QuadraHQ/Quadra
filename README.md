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
  A bootstrapper for the macOS Roblox Player client, running on Linux through <a href="https://github.com/microlift/rosemary">Rosemary</a>.
</p>

## EXPERIMENTAL WARNING

This is a rewrite of Quadra in Rust. This hasn't been extensively tested yet, and there are some missing features, including:

- Server browser
- VR
- Discord RPC
- Roblox Studio
- Support for the Darling runtime

## Build instructions

```
git clone https://github.com/QuadraHQ/quadra-rewrite && cd quadra-rewrite
cargo build
```

## Relevant directories

- `~/.local/share/quadra`: Where Quadra stores its data
- `~/.local/share/quadra/prefix`: The Rosemary prefix Roblox runs in
- `~/.local/share/quadra/rosemary/<version>`: Installed Rosemary versions
- `~/.local/share/quadra/roblox`: The Roblox Player installation
- `~/.local/share/quadra/logs`: Quadra and Roblox logs
- `~/.cache/quadra/downloads`: Downloaded files, such as Rosemary and Roblox packages
- `~/.local/share/butter/runners` and `~/.var/app/io.github.microlift.butter/data/butter/runners`: Butter runner directories that Quadra also detects runners from
