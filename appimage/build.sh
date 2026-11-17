#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$(realpath "$0")")"

tools="$PWD/tools"
appdir="$PWD/AppDir"

download_tool() {
    local name="$1" url="$2"
    if [ ! -x "$tools/$name" ]; then
        curl -fsSL -o "$tools/$name" "$url"
        chmod +x "$tools/$name"
    fi
}

mkdir -p "$tools"
download_tool linuxdeploy-x86_64.AppImage \
    https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage

cargo build --release --manifest-path ../Cargo.toml

export PATH="$tools:$PATH"

icon_files=()
for icon in ../data/icons/hicolor/*/apps/org.quadrahq.Quadra.png; do
    icon_files+=(--icon-file "$icon")
done

rm -rf "$appdir"
linuxdeploy-x86_64.AppImage \
    --appdir "$appdir" \
    --executable ../target/release/quadra \
    --desktop-file ../data/org.quadrahq.Quadra.desktop \
    "${icon_files[@]}"

linuxdeploy-x86_64.AppImage --appdir "$appdir" --output appimage
