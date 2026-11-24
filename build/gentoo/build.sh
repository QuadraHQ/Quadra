#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$(realpath "$0")")"

version="$(sed -n 's/^version = "\(.*\)"/\1/p' ../../Cargo.toml | head -n1)"
name="quadra-$version.ebuild"

crates="$(awk '
    $1 == "name" { crate = $3; gsub(/"/, "", crate) }
    $1 == "version" { number = $3; gsub(/"/, "", number) }
    $1 == "source" && $3 ~ /registry/ { print "\t" crate "-" number }
' ../../Cargo.lock | LC_ALL=C sort -u)"

rm -f quadra-*.ebuild
while IFS= read -r line; do
    if [[ "$line" == "@CRATES@" ]]; then
        printf '%s\n' "$crates"
    else
        printf '%s\n' "$line"
    fi
done < quadra.ebuild.in > "$name"
