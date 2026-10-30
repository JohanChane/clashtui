#!/bin/sh
# Usage: service.sh OP system|user SERVICE
set -u
export LC_ALL=C
fail() { printf '%s\n' "$*" >&2; exit 1; }
[ "$#" -eq 3 ] || fail 'Expected OP SCOPE SERVICE'
op=$1 scope=$2 service=$3
case "$op" in start|stop|restart|status|enable-autostart|disable-autostart) ;; *) fail "Unknown operation: $op";; esac
case "$service" in ''|-*|*[!a-zA-Z0-9_.@:-]*) fail 'Invalid service name';; esac
command -v launchctl >/dev/null 2>&1 || fail 'launchctl is not available'
case "$scope" in
    system) domain=system; directory=/Library/LaunchDaemons;;
    user) domain="gui/$(id -u)"; directory="${HOME:?HOME is required}/Library/LaunchAgents";;
    *) fail "Unknown scope: $scope";;
esac
target=$domain/$service
plist=$directory/$service.plist
privileged() {
    if [ "$scope" = user ] || [ "$(id -u)" -eq 0 ]; then
        "$@"
    else
        sudo -- "$@"
    fi
}
ctl() {
    # Read-only queries do not require system privileges or a sudo timestamp.
    if [ "$1" = print ]; then launchctl "$@"; else privileged launchctl "$@"; fi
}
inspect_plist() {
    # Unreadable paths are query failures, not evidence that a service is absent.
    probe=$directory
    # Check the nearest existing ancestor before treating an absent path as missing.
    while [ ! -e "$probe" ] && [ ! -L "$probe" ]; do
        parent=${probe%/*}
        [ "$parent" != "$probe" ] || fail "Cannot inspect $directory"
        probe=${parent:-/}
    done
    [ -d "$probe" ] && [ -r "$probe" ] && [ -x "$probe" ] || fail "Cannot inspect $directory"
    if [ -f "$plist" ]; then
        [ -r "$plist" ] || fail "Cannot read $plist"
        printf 'inactive\n'
    else printf 'missing\n'; fi
}

query() {
    loaded=false
    if details=$(ctl print "$target" 2>&1); then
        loaded=true
        case "$details" in
            *'state = running'*) state=active;;
            *'state = '*) state=inactive;;
            *) fail 'Unrecognized launchctl print output';;
        esac
        return
    fi
    # A failed query is not evidence of absence until its domain is accessible
    # and launchctl explicitly identifies the service as absent.
    case "$details" in *'Could not find service'*|*'Could not find specified service'*) ;; *) fail "$details";; esac
    ctl print "$domain" >/dev/null || fail "Cannot query launchd domain: $domain"
    state=$(inspect_plist) || fail "Cannot inspect $plist"
}
unload() {
    ctl unload "$plist" || fail "Could not unload $plist"
    query
    [ "$loaded" = false ] || fail 'launchctl unload left the service loaded'
}
load() {
    file_state=$(inspect_plist) || fail "Cannot inspect $plist"
    [ "$file_state" = inactive ] || fail "Cannot read $plist"
    # Force this manual load without changing the persistent autostart override.
    ctl load -F "$plist" || fail 'launchctl load failed'
    query
    attempts=0
    while [ "$loaded" = true ] && [ "$state" != active ] && [ "$attempts" -lt 5 ]; do
        sleep 1
        attempts=$((attempts + 1))
        query
    done
    [ "$state" = active ] || fail 'Service did not start; check its plist and core logs'
}
query
case "$op" in
    status) printf '%s\n' "$state";;
    stop) [ "$loaded" = false ] || unload;;
    start)
        [ "$state" != missing ] || fail "Service is not installed: $service"
        if [ "$state" != active ]; then
            [ "$loaded" = false ] || unload
            load
        fi;;
    restart)
        [ "$state" != missing ] || fail "Service is not installed: $service"
        [ "$loaded" = false ] || unload
        load;;
    enable-autostart|disable-autostart)
        [ "$state" != missing ] || fail "Service is not installed: $service"
        case "$op" in enable-autostart) ctl enable "$target";; *) ctl disable "$target";; esac;;
esac
