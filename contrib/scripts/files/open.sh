#!/bin/sh
# Open an absolute file or directory path with the desktop's default application.
set -eu
if [ "$#" -ne 1 ] || [ -z "$1" ]; then
    echo 'Usage: open.sh PATH' >&2
    exit 2
fi
case "$1" in
    /*) ;;
    *) echo 'open.sh: PATH must be absolute' >&2; exit 2 ;;
esac
case "$(uname -s)" in
    Darwin) exec open "$1" ;;
    *)
        if [ -z "${DISPLAY:-}" ] && [ -z "${WAYLAND_DISPLAY:-}" ]; then
            echo 'No desktop session; configure edit_file/open_dir with interactive: true for a terminal tool' >&2
            exit 1
        fi
        exec xdg-open "$1"
        ;;
esac
