#!/bin/sh
# Read UTF-8 text verbatim from stdin. Native backends leave stdout empty.
# OSC 52 is written to stdout for the caller to forward to the terminal.
set -eu

backend=${1:-auto}
if [ "$#" -gt 1 ]; then
    echo "Usage: copy.sh [auto|osc52|pbcopy|wl-copy|xclip|xsel]" >&2
    exit 2
fi
if [ "$backend" = auto ]; then
    if [ -n "${SSH_TTY:-}${SSH_CONNECTION:-}${SSH_CLIENT:-}" ]; then
        backend=osc52
    elif [ "$(uname -s)" = Darwin ]; then
        backend=pbcopy
    elif [ -n "${WAYLAND_DISPLAY:-}" ] && command -v wl-copy >/dev/null 2>&1; then
        backend=wl-copy
    elif [ -n "${DISPLAY:-}" ] && command -v xclip >/dev/null 2>&1; then
        backend=xclip
    elif [ -n "${DISPLAY:-}" ] && command -v xsel >/dev/null 2>&1; then
        backend=xsel
    else
        backend=osc52
    fi
fi
case "$backend" in
    osc52)
        # Encode stdin directly: shell command substitution must not strip the
        # original text's trailing newlines. Only remove base64 line wrapping.
        encoded=$(base64)
        encoded=$(printf '%s' "$encoded" | tr -d '\r\n')
        printf '\033]52;c;%s\007' "$encoded"
        ;;
    pbcopy) exec pbcopy ;;
    wl-copy) exec wl-copy --type 'text/plain;charset=utf-8' ;;
    xclip) exec xclip -selection clipboard -in ;;
    xsel) exec xsel --clipboard --input ;;
    *) echo "Unknown clipboard backend: $backend" >&2; exit 2 ;;
esac
