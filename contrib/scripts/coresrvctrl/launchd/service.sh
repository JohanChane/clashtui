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
query() {
    loaded=false
    if details=$(launchctl print "$target" 2>&1); then
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
    launchctl print "$domain" >/dev/null || fail "Cannot query launchd domain: $domain"
    case "$details" in *'Could not find service'*|*'Could not find specified service'*) ;; *) fail "$details";; esac
    [ ! -d "$directory" ] || { [ -r "$directory" ] && [ -x "$directory" ]; } || fail "Cannot inspect $directory"
    if [ -f "$plist" ]; then
        [ -r "$plist" ] || fail "Cannot read $plist"
        state=inactive
    else state=missing; fi
}
unload() {
    launchctl unload "$plist" || fail "Could not unload $plist"
    query
    [ "$loaded" = false ] || fail 'launchctl unload left the service loaded'
}
load() {
    [ -r "$plist" ] || fail "Cannot read $plist"
    launchctl load "$plist" || fail 'launchctl load failed; the service may be disabled'
    query
    attempts=0
    while [ "$loaded" = true ] && [ "$state" != active ] && [ "$attempts" -lt 5 ]; do
        sleep 1
        attempts=$((attempts + 1))
        query
    done
    [ "$state" = active ] || fail 'Service did not start; check its config and launchctl enable/disable policy'
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
        case "$op" in enable-autostart) launchctl enable "$target";; *) launchctl disable "$target";; esac;;
esac
