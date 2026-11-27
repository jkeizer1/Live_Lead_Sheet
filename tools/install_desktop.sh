#!/bin/bash
# Makes a double-click on a .song file open it in Live Lead Sheet.
#
#   tools/install_desktop.sh            builds the release program and installs
#   tools/install_desktop.sh uninstall  removes what install made
#
# It writes only under ~/.local/share (your own account, no sudo):
#   applications/live-lead-sheet.desktop   the launcher, pointing at this
#                                          project's tools/open_song.sh
#   mime/packages/live-lead-sheet.xml      says *.song is a Live Lead Sheet song
# and makes the launcher the default for that type. open_song.sh builds the
# program before starting it, so a code change needs nothing more. If the
# project folder moves, run install again.

set -eu

project="$(cd "$(dirname "$(readlink -f "${BASH_SOURCE[0]}")")/.." && pwd)"
data="${XDG_DATA_HOME:-$HOME/.local/share}"
applications="$data/applications"
mime="$data/mime"

die() {
    echo "Error: $*" >&2
    exit 1
}

refresh() {
    update-mime-database "$mime" || die "update-mime-database failed."
    if command -v update-desktop-database >/dev/null; then
        update-desktop-database "$applications" || die "update-desktop-database failed."
    fi
}

case "${1:-install}" in
install)
    command -v update-mime-database >/dev/null ||
        die "update-mime-database is missing (package shared-mime-info)."
    command -v xdg-mime >/dev/null || die "xdg-mime is missing (package xdg-utils)."
    cargo build --release --manifest-path "$project/Cargo.toml" -p app ||
        die "the release build failed."
    program="$project/target/release/app"
    [[ -x "$program" ]] || die "$program was not made; is CARGO_TARGET_DIR set?"
    mkdir -p "$applications" "$mime/packages"
    sed "s|@PROJECT@|$project|g" "$project/tools/live-lead-sheet.desktop.in" \
        >"$applications/live-lead-sheet.desktop"
    cp "$project/tools/live-lead-sheet-mime.xml" "$mime/packages/live-lead-sheet.xml"
    refresh
    xdg-mime default live-lead-sheet.desktop text/x-live-lead-sheet-song
    echo "Installed. Double-click a .song file to open it."
    echo "Launcher: $applications/live-lead-sheet.desktop"
    ;;
uninstall)
    rm -f "$applications/live-lead-sheet.desktop" "$mime/packages/live-lead-sheet.xml"
    refresh
    echo "Removed."
    ;;
*)
    die "usage: tools/install_desktop.sh [install|uninstall]"
    ;;
esac
