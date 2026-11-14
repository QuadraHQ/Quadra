#!/usr/bin/env bash
set -euo pipefail

REPOSITORY=${QUADRA_REPOSITORY:-QuadraHQ/quadra}
API_URL=${QUADRA_API_URL:-https://api.github.com}
DOWNLOAD_URL=${QUADRA_DOWNLOAD_URL:-https://github.com}
FLATHUB_REPOSITORY=https://dl.flathub.org/repo/flathub.flatpakrepo
FLATPAK_ID=org.quadrahq.Quadra
DESKTOP_FILE_NAME=org.quadrahq.Quadra.desktop

BIN_HOME=${XDG_BIN_HOME:-$HOME/.local/bin}
DATA_HOME=${XDG_DATA_HOME:-$HOME/.local/share}
CONFIG_HOME=${XDG_CONFIG_HOME:-$HOME/.config}
CACHE_HOME=${XDG_CACHE_HOME:-$HOME/.cache}
APPIMAGE_PATH=$BIN_HOME/Quadra.AppImage
NATIVE_PATH=$BIN_HOME/quadra
SOURCE_DIRECTORY=$DATA_HOME/quadra/source

MINIMUM_GTK=4.18
MINIMUM_LIBADWAITA=1.7
MINIMUM_RUST_MINOR=85

mode=appimage
action=install
purge=0
tag=
temporary_path=
temporary_files=()

usage() {
    cat <<'USAGE'
Quadra installer

  curl -fsSL https://quadrahq.org/install.sh | bash
  curl -fsSL https://quadrahq.org/install.sh | bash -s -- [options]

Installs the newest Quadra as an AppImage in ~/.local/bin and adds it to the
app menu. Run it again to update.

Options:
  --flatpak        install the Flatpak bundle instead (needs flatpak)
  --tarball        install the prebuilt binary instead (needs GTK 4.18 and
                   libadwaita 1.7 on this computer)
  --native         build Quadra on this computer instead (needs git, Rust 1.85,
                   a C compiler, GTK 4.18 and libadwaita 1.7 with headers)
  --version TAG    install that release, for example v0.50.0-alpha.3
  --uninstall      remove Quadra but keep its data and your Roblox sign-in
  --purge          with --uninstall, also delete Quadra's data and sign-in
  -h, --help       show this help
USAGE
}

say() { printf '  %s\n' "$*"; }
fail() { printf 'error: %s\n' "$*" >&2; exit 1; }

cleanup() {
    local file
    for file in "${temporary_files[@]+"${temporary_files[@]}"}"; do
        rm -rf -- "$file"
    done
}

need() {
    command -v "$1" >/dev/null 2>&1 || fail "$1 is needed. $2"
}

make_temporary() {
    temporary_path=$(mktemp "$@") || fail "could not create a temporary file"
    temporary_files+=("$temporary_path")
}

check_machine() {
    [ "$(uname -m)" = x86_64 ] || fail "Quadra only has x86_64 builds, and this is $(uname -m)."
    need curl "Install it with your package manager and run this again."
}

latest_tag() {
    local listing found
    listing=$(curl -fsSL -H 'Accept: application/vnd.github+json' \
        "$API_URL/repos/$REPOSITORY/releases?per_page=10") \
        || fail "could not look up Quadra's releases. Check your connection."
    found=$(printf '%s\n' "$listing" \
        | grep -o '"tag_name"[[:space:]]*:[[:space:]]*"[^"]*"' | head -n 1 \
        | sed 's/.*:[[:space:]]*"\(.*\)"/\1/')
    [ -n "$found" ] || fail "no Quadra release was found."
    printf '%s\n' "$found"
}

check_tag() {
    printf '%s\n' "$1" | grep -Eq '^v[0-9]+\.[0-9]+\.[0-9]+([-+][0-9A-Za-z.-]+)?$' \
        || fail "'$1' is not a release tag like v0.50.0-alpha.3."
}

release_url() {
    printf '%s/%s/releases/download/%s' "$DOWNLOAD_URL" "$REPOSITORY" "$tag"
}

download_verified() {
    local name=$1 destination=$2 checksums expected actual partial
    checksums=$(curl -fsSL "$(release_url)/SHA256SUMS") \
        || fail "$tag has no SHA256SUMS, so Quadra will not install it."
    expected=$(printf '%s\n' "$checksums" | awk -v name="$name" \
        '{ file = $2; sub(/^\*/, "", file) } file == name { print $1; exit }')
    [ -n "$expected" ] || fail "the checksums of $tag do not list $name."

    make_temporary "$destination.XXXXXX"
    partial=$temporary_path
    curl -fL --progress-bar -o "$partial" "$(release_url)/$name" \
        || fail "could not download $name."
    actual=$(sha256sum "$partial" | cut -d' ' -f1)
    [ "$actual" = "$expected" ] || fail "$name does not match its checksum, so it was not installed."
    chmod 755 "$partial"
    mv -f -- "$partial" "$destination"
}

quote_for_exec() {
    printf '"%s"' "$(printf '%s' "$1" | sed 's/\\/\\\\\\\\/g; s/"/\\"/g; s/`/\\`/g; s/\$/\\$/g')"
}

write_menu_entry() {
    local executable=$1 icon=$DATA_HOME/quadra/icons/org.quadrahq.Quadra.png
    mkdir -p "$DATA_HOME/applications"
    cat > "$DATA_HOME/applications/$DESKTOP_FILE_NAME" <<ENTRY
[Desktop Entry]
Type=Application
Name=Quadra
Comment=Play Roblox on Linux
Exec=$(quote_for_exec "$executable")
Icon=$([ -f "$icon" ] && printf '%s' "$icon" || printf '%s' org.quadrahq.Quadra)
Terminal=false
Categories=Game;
StartupNotify=true
ENTRY
}

extract_icon() {
    local work source
    make_temporary -d
    work=$temporary_path
    (cd "$work" && "$APPIMAGE_PATH" --appimage-extract 'usr/share/icons/hicolor/*/apps/org.quadrahq.Quadra.png' \
        >/dev/null 2>&1) || return 0
    source=$(find "$work" -name 'org.quadrahq.Quadra.png' -print -quit 2>/dev/null)
    [ -n "$source" ] || return 0
    mkdir -p "$DATA_HOME/quadra/icons"
    install -m 644 -- "$source" "$DATA_HOME/quadra/icons/org.quadrahq.Quadra.png"
}

install_appimage() {
    mkdir -p "$BIN_HOME"
    say "Downloading Quadra $tag"
    download_verified "Quadra-$tag-x86_64.AppImage" "$APPIMAGE_PATH"
    extract_icon
    write_menu_entry "$APPIMAGE_PATH"
    say "Installed $APPIMAGE_PATH"
    if ! command -v fusermount >/dev/null 2>&1 && ! command -v fusermount3 >/dev/null 2>&1; then
        say "AppImages need FUSE. Install fuse2 (libfuse2), or start Quadra with APPIMAGE_EXTRACT_AND_RUN=1."
    fi
}

install_flatpak() {
    need flatpak "Install it with your package manager and run this again."
    local bundle
    mkdir -p "$CACHE_HOME/quadra"
    bundle=$CACHE_HOME/quadra/Quadra-$tag-x86_64.flatpak
    say "Downloading Quadra $tag"
    download_verified "Quadra-$tag-x86_64.flatpak" "$bundle"
    flatpak remote-add --user --if-not-exists flathub "$FLATHUB_REPOSITORY" \
        || fail "could not add Flathub, which the Flatpak needs for its runtime."
    flatpak install --user --bundle -y --noninteractive "$bundle" \
        || fail "the Flatpak did not install."
    rm -f -- "$bundle"
    say "Installed the Flatpak. Start it from the app menu or with: flatpak run $FLATPAK_ID"
}

install_tarball() {
    need tar "Install it with your package manager and run this again."
    local archive work unpacked missing
    mkdir -p "$CACHE_HOME/quadra" "$BIN_HOME" "$DATA_HOME/quadra/icons"
    archive=$CACHE_HOME/quadra/Quadra-$tag-x86_64.tar.gz
    say "Downloading Quadra $tag"
    download_verified "Quadra-$tag-x86_64.tar.gz" "$archive"

    make_temporary -d
    work=$temporary_path
    tar -xzf "$archive" -C "$work" || fail "could not unpack Quadra-$tag-x86_64.tar.gz."
    unpacked=$work/Quadra-x86_64
    [ -f "$unpacked/bin/quadra" ] || fail "Quadra-$tag-x86_64.tar.gz has no Quadra in it."

    install -m 755 -- "$unpacked/bin/quadra" "$NATIVE_PATH"
    install -m 644 -- "$unpacked/share/icons/hicolor/256x256/apps/org.quadrahq.Quadra.png" \
        "$DATA_HOME/quadra/icons/org.quadrahq.Quadra.png"
    rm -f -- "$archive"
    write_menu_entry "$NATIVE_PATH"
    say "Installed $NATIVE_PATH"

    if command -v ldd >/dev/null 2>&1; then
        missing=$(ldd "$NATIVE_PATH" 2>/dev/null | awk '/not found/ { print $1 }')
        if [ -n "$missing" ]; then
            say "Quadra won't start until these libraries are installed:"
            printf '%s\n' "$missing" | sed 's/^/    - /'
            say "Install GTK $MINIMUM_GTK and libadwaita $MINIMUM_LIBADWAITA or newer, or use the AppImage."
        fi
    fi
}

native_problems() {
    local program cargo_minor
    for program in git cargo cc pkg-config; do
        command -v "$program" >/dev/null 2>&1 || printf '%s is not installed\n' "$program"
    done
    if command -v pkg-config >/dev/null 2>&1; then
        pkg-config --atleast-version="$MINIMUM_GTK" gtk4 2>/dev/null \
            || printf 'GTK %s or newer, with its headers, is needed\n' "$MINIMUM_GTK"
        pkg-config --atleast-version="$MINIMUM_LIBADWAITA" libadwaita-1 2>/dev/null \
            || printf 'libadwaita %s or newer, with its headers, is needed\n' "$MINIMUM_LIBADWAITA"
    fi
    if command -v cargo >/dev/null 2>&1; then
        cargo_minor=$(cargo --version 2>/dev/null | sed -n 's/^cargo 1\.\([0-9]*\)\..*/\1/p')
        [ "${cargo_minor:-0}" -ge "$MINIMUM_RUST_MINOR" ] \
            || printf 'Rust 1.%s or newer is needed\n' "$MINIMUM_RUST_MINOR"
    fi
}

install_native() {
    local problems
    problems=$(native_problems)
    if [ -n "$problems" ]; then
        printf 'This computer cannot build Quadra yet:\n' >&2
        printf '%s\n' "$problems" | sed 's/^/  - /' >&2
        fail "install those, or use the AppImage."
    fi

    if [ -d "$SOURCE_DIRECTORY/.git" ]; then
        git -C "$SOURCE_DIRECTORY" fetch --depth 1 origin "refs/tags/$tag:refs/tags/$tag" \
            && git -C "$SOURCE_DIRECTORY" checkout -q "$tag" \
            || fail "could not fetch Quadra $tag."
    else
        mkdir -p "$(dirname "$SOURCE_DIRECTORY")"
        git clone --depth 1 --branch "$tag" "$DOWNLOAD_URL/$REPOSITORY.git" "$SOURCE_DIRECTORY" \
            || fail "could not download Quadra $tag."
    fi

    say "Building Quadra $tag. This takes a few minutes."
    (cd "$SOURCE_DIRECTORY" && cargo build --release --locked) || fail "the build failed."
    mkdir -p "$BIN_HOME"
    install -m 755 "$SOURCE_DIRECTORY/target/release/quadra" "$NATIVE_PATH"
    write_menu_entry "$NATIVE_PATH"
    say "Built and installed $NATIVE_PATH"
}

shortcuts_running() {
    local executable=$1 file
    for file in "$DATA_HOME"/applications/org.quadrahq.Quadra*.desktop; do
        if [ -f "$file" ] && grep -q "^Exec=.*$executable" "$file"; then
            printf '%s\n' "$file"
        fi
    done
    return 0
}

uninstall() {
    local executable file
    for executable in "$APPIMAGE_PATH" "$NATIVE_PATH"; do
        shortcuts_running "$executable" | while IFS= read -r file; do rm -f -- "$file"; done
        if [ -e "$executable" ]; then
            rm -f -- "$executable"
            say "Removed $executable"
        fi
    done
    if command -v flatpak >/dev/null 2>&1 && flatpak info --user "$FLATPAK_ID" >/dev/null 2>&1; then
        if flatpak uninstall --user -y --noninteractive "$FLATPAK_ID"; then
            say "Removed the Flatpak"
        else
            say "Could not remove the Flatpak. Remove it with: flatpak uninstall $FLATPAK_ID"
        fi
    fi
    if [ "$purge" -eq 1 ]; then
        rm -rf -- "$DATA_HOME/quadra" "$CONFIG_HOME/quadra" "$CACHE_HOME/quadra"
        say "Deleted Quadra's data, settings and Roblox sign-in"
    else
        say "Kept $DATA_HOME/quadra (your Roblox sign-in). Add --purge to delete it."
    fi
}

mention_old_quadra() {
    if [ -f "$DATA_HOME/QuadraHQ/launcher/quadrahq-launcher" ]; then
        say "The old Quadra is still installed. Once the new one works, you can delete it to free space: rm -rf $DATA_HOME/QuadraHQ"
    fi
}

mention_path() {
    case ":$PATH:" in
        *":$BIN_HOME:"*) ;;
        *) say "Add $BIN_HOME to your PATH to start it from a terminal." ;;
    esac
}

parse_arguments() {
    while [ $# -gt 0 ]; do
        case $1 in
            --flatpak) mode=flatpak ;;
            --tarball) mode=tarball ;;
            --native) mode=native ;;
            --uninstall) action=uninstall ;;
            --purge) purge=1 ;;
            --version)
                [ $# -ge 2 ] || fail "--version needs a release tag."
                tag=$2
                shift
                ;;
            -h | --help)
                usage
                exit 0
                ;;
            *) fail "unknown option $1. See --help." ;;
        esac
        shift
    done
}

main() {
    trap cleanup EXIT
    parse_arguments "$@"

    if [ "$action" = uninstall ]; then
        uninstall
        exit 0
    fi

    check_machine
    need sha256sum "It comes with coreutils."
    if [ -z "$tag" ]; then
        tag=$(latest_tag)
    fi
    check_tag "$tag"

    case $mode in
        appimage) install_appimage ;;
        flatpak) install_flatpak ;;
        tarball) install_tarball ;;
        native) install_native ;;
    esac
    mention_old_quadra
    mention_path
    say "Open Quadra from your app menu."
}

main "$@"
exit
