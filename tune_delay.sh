#!/bin/bash
# Measures the PC audio latency and writes it into the newest .studio file next
# to this script, as `delay_trigger N ms` on every track not on the port
# "DistingNT". See tools/tune_delay.py for what it measures and how.
#
# Use it from a song folder: copy it there, or make a link to it:
#   ln -s ~/Documents/Live_Lead_Sheet/tune_delay.sh ~/Documents/Live_Lead_Sheet_MySongs/Nov20_Delay/tune_delay.sh
# then run ./tune_delay.sh in that folder with the audio flowing. Options go on
# after it: --runs 9, --dry-run, --set-ms 24, --port-skipped NAME.
#
# The project folder is found from the link, else from LIVE_LEAD_SHEET_DIR,
# else it is ~/Documents/Live_Lead_Sheet. The terminal stays open at a shell
# prompt in the song folder afterwards.

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
    if [[ -f "$linked/tools/tune_delay.py" ]]; then
        project="$linked"
    else
        project="$HOME/Documents/Live_Lead_Sheet"
    fi
fi
[[ -f "$project/tools/tune_delay.py" ]] ||
    die "the Live_Lead_Sheet project (tools/tune_delay.py) is not at $project. Set LIVE_LEAD_SHEET_DIR to its folder."

# The studio file saved last in this folder.
count=$(find "$here" -maxdepth 1 -name '*.studio' -type f | wc -l)
[[ "$count" -ge 1 ]] || die "$here has no .studio file."
studio="$(ls -t "$here"/*.studio | head -n 1)"
if [[ "$count" -gt 1 ]]; then
    echo "Using the newest of $count .studio files: $(basename "$studio")" >&2
fi

python3 "$project/tools/tune_delay.py" "$studio" "$@"

stay_open
