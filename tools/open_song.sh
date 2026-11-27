#!/bin/bash
# Opens a song in Live Lead Sheet, building the program first, so a
# double-click always runs the code as it stands. The launcher made by
# tools/install_desktop.sh starts this script.
#
#   tools/open_song.sh [SONG_FILE]
#
# The build's output goes to target/open_song_build.log. A build that takes
# longer than 1 s shows a "Building" window. When the build fails, a window
# shows the end of the log and the program is not started: an older build
# would hide the change being tested.

set -u

project="$(cd "$(dirname "$(readlink -f "${BASH_SOURCE[0]}")")/.." && pwd)"
log="$project/target/open_song_build.log"
program="$project/target/release/app"

# A launcher started from the file manager may not have cargo's folder on
# its PATH.
PATH="$PATH:$HOME/.cargo/bin"

show_error() {
    if command -v zenity >/dev/null; then
        zenity --error --no-markup --width=700 --title="Live Lead Sheet" --text="$1"
    else
        notify-send "Live Lead Sheet" "$1"
    fi
}

command -v cargo >/dev/null || {
    show_error "cargo not found, so the program was not built or started. Install Rust (rustup), or add cargo's folder to PATH."
    exit 1
}

mkdir -p "$project/target"
cargo build --release --manifest-path "$project/Cargo.toml" -p app >"$log" 2>&1 &
build=$!

# Nothing to show for a build that only checks dates: wait up to 1 s
# (10 × 0.1 s) before opening the "Building" window.
for _ in 1 2 3 4 5 6 7 8 9 10; do
    kill -0 "$build" 2>/dev/null || break
    sleep 0.1
done

progress=""
if kill -0 "$build" 2>/dev/null && command -v zenity >/dev/null; then
    # zenity pulses while its input stays open; it is closed when the build ends.
    zenity --progress --pulsate --no-cancel --title="Live Lead Sheet" \
        --text="Building Live Lead Sheet…" < <(while sleep 1; do echo "#Building Live Lead Sheet…"; done) &
    progress=$!
fi

wait "$build"
status=$?
if [[ -n "$progress" ]]; then
    kill "$progress" 2>/dev/null
fi

if [[ "$status" -ne 0 ]]; then
    show_error "The program did not build, so it was not started. Fix the error below, then open the song again.

$(tail -n 15 "$log")

Full log: $log"
    exit 1
fi

[[ -x "$program" ]] || {
    show_error "The build finished but $program is missing. Is CARGO_TARGET_DIR set? Unset it, then open the song again."
    exit 1
}

exec "$program" "$@"
