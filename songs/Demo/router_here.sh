#!/bin/bash
# Opens the MIDI router window with the newest .toml file next to this script.
#
# Use it from a song folder that keeps its router file beside the song: copy it
# there, or make a link to it:
#   ln -s ~/Documents/Linux_MIDI_Router/router_here.sh ~/Documents/Live_Lead_Sheet_MySongs/Nov20_Delay/router_here.sh
# then run ./router_here.sh in that folder (or double-click it).
#
# The router's folder is found from the link, else from LINUX_MIDI_ROUTER_DIR,
# else it is ~/Documents/Linux_MIDI_Router. Closing the window stops the router;
# the terminal then stays open at a shell prompt in the song folder.

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

router="${LINUX_MIDI_ROUTER_DIR:-}"
if [[ -z "$router" ]]; then
    linked="$(dirname "$(readlink -f "${BASH_SOURCE[0]}")")"
    if [[ -d "$linked/midi_router" ]]; then
        router="$linked"
    else
        router="$HOME/Documents/Linux_MIDI_Router"
    fi
fi
[[ -d "$router/midi_router" ]] ||
    die "the Linux_MIDI_Router project is not at $router. Set LINUX_MIDI_ROUTER_DIR to its folder."

shopt -s nullglob
files=("$here"/*.toml)
[[ ${#files[@]} -ge 1 ]] || die "$here has no .toml file."
configuration="$(ls -t "${files[@]}" | head -n 1)"
if [[ ${#files[@]} -gt 1 ]]; then
    echo "Using the newest of ${#files[@]} .toml files: $(basename "$configuration")" >&2
fi

# The package is run from its own folder, wherever the song folder is.
(cd "$router" && PYTHONPATH="$router" python3 -m midi_router.window "$configuration")

stay_open
