#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$(realpath "$0")")"

name=Quadra-x86_64
staging="$PWD/staging"
root="$staging/$name"

cargo build --release --locked --manifest-path ../Cargo.toml

rm -rf "$staging" "$name.tar.gz"
install -Dm755 ../target/release/quadra "$root/bin/quadra"
install -Dm644 ../data/org.quadrahq.Quadra.desktop \
    "$root/share/applications/org.quadrahq.Quadra.desktop"
mkdir -p "$root/share/icons"
cp -r ../data/icons/hicolor "$root/share/icons/"
install -Dm644 ../LICENSE "$root/LICENSE"

tar --sort=name --owner=0 --group=0 --numeric-owner \
    -C "$staging" -czf "$name.tar.gz" "$name"
rm -rf "$staging"
