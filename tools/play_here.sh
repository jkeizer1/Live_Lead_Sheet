#!/bin/bash
# Plays the song in this folder: the newest .song and the newest .studio file
# next to this script, through song_play_test.
#
# Use it from a song folder: copy it there, or make a link to it:
#   ln -s ~/Documents/Live_Lead_Sheet/play_here.sh ~/Documents/Live_Lead_Sheet_MySongs/Nov20_Delay/play_here.sh
# then run ./play_here.sh in that folder (or double-click it).
#
# The project folder is found from the link, else from LIVE_LEAD_SHEET_DIR,
# else it is ~/Documents/Live_Lead_Sheet.

set -u

# Leaves the terminal at a shell prompt in this folder, for more commands. When
# the script was started from a shell you already have open, that shell is
# simply still there; when a file manager or a terminal launcher started the
# script itself, the window would close, so a new shell takes its place.
stay_open() {
    local parent
    parent="$(ps -o comm= -p "$PPID" 2>/dev/null | tr -d ' ')"
    case "$parent" in
        bash | zsh | sh | dash | fish | ksh) return 0 ;;
    esac
    cd "${here:-.}" 2>/dev/null || true
    echo
    exec "${SHELL:-/bin/bash}" -i
}

die() {
    echo >&2
    echo "Error: $*" >&2
    stay_open
    exit 1
}

# The folder the script is run from, not where a link points.
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)" || die "cannot find this script's folder."

project="${LIVE_LEAD_SHEET_DIR:-}"
if [[ -z "$project" ]]; then
    linked="$(dirname "$(readlink -f "${BASH_SOURCE[0]}")")"
    if [[ -f "$linked/Cargo.toml" ]]; then
        project="$linked"
    else
        project="$HOME/Documents/Live_Lead_Sheet"
    fi
fi
[[ -f "$project/Cargo.toml" ]] ||
    die "the Live_Lead_Sheet project is not at $project. Set LIVE_LEAD_SHEET_DIR to its folder."

# The newest file with an extension: with several .song or .studio files in the
# folder, the one saved last is played. The name is shown when there was a choice.
newest() {
    local extension="$1" count latest
    count=$(find "$here" -maxdepth 1 -name "*.$extension" -type f | wc -l)
    # Not die: this runs in a subshell, which must not start a shell of its own.
    if [[ "$count" -lt 1 ]]; then
        printf '\nError: %s has no .%s file.\n' "$here" "$extension" >&2
        return 1
    fi
    latest=$(ls -t "$here"/*."$extension" | head -n 1)
    if [[ "$count" -gt 1 ]]; then
        echo "Playing the newest of $count .$extension files: $(basename "$latest")" >&2
    fi
    printf '%s\n' "$latest"
}

song="$(newest song)" || { stay_open; exit 1; }
studio="$(newest studio)" || { stay_open; exit 1; }

cargo run --release --manifest-path "$project/Cargo.toml" \
    -p song_text --example song_play_test -- "$song" "$studio"

stay_open
