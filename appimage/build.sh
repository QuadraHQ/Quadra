#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$(realpath "$0")")"

tools="$PWD/tools"
appdir="$PWD/AppDir"
gtk_hook="$appdir/apprun-hooks/linuxdeploy-plugin-gtk.sh"

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
download_tool linuxdeploy-plugin-gtk.sh \
    https://raw.githubusercontent.com/linuxdeploy/linuxdeploy-plugin-gtk/master/linuxdeploy-plugin-gtk.sh

cargo build --release --manifest-path ../Cargo.toml

export PATH="$tools:$PATH"
export DEPLOY_GTK_VERSION=4

rm -rf "$appdir"
linuxdeploy-x86_64.AppImage \
    --appdir "$appdir" \
    --executable ../target/release/quadra \
    --desktop-file ../data/org.quadrahq.Quadra.desktop \
    --icon-file ../data/icons/org.quadrahq.Quadra.png \
    --plugin gtk

sed -i '/^export GTK_THEME=/d; /^export GDK_BACKEND=/d' "$gtk_hook"

linuxdeploy-x86_64.AppImage --appdir "$appdir" --output appimage
