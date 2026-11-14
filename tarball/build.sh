#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$(realpath "$0")")"

name=Quadra-x86_64
staging="$PWD/staging"
root="$staging/$name"

cargo build --release --locked --manifest-path ../Cargo.toml

rm -rf "$staging" "$name.tar.gz"
install -Dm755 ../target/release/quadra-rewrite "$root/bin/quadra-rewrite"
install -Dm644 ../data/org.quadrahq.Quadra.desktop \
    "$root/share/applications/org.quadrahq.Quadra.desktop"
install -Dm644 ../data/icons/org.quadrahq.Quadra.png \
    "$root/share/icons/hicolor/256x256/apps/org.quadrahq.Quadra.png"
install -Dm644 ../LICENSE "$root/LICENSE"

tar --sort=name --owner=0 --group=0 --numeric-owner \
    -C "$staging" -czf "$name.tar.gz" "$name"
rm -rf "$staging"
