#!/bin/sh
# Usage: service.sh OP system|user SERVICE

fail() {
    printf '%s\n' "$*" >&2
    exit 1
}

parse_arguments() {
    [ "$#" -eq 3 ] || fail 'Expected OP SCOPE SERVICE'
    operation=$1
    scope=$2
    service=$3

    case "$operation" in
        start|stop|restart|status|enable-autostart|disable-autostart) ;;
        *) fail "Unknown operation: $operation" ;;
    esac
    case "$scope" in
        system|user) ;;
        *) fail "Unknown scope: $scope" ;;
    esac
    case "$service" in
        ''|-*|*[!a-zA-Z0-9_.@:-]*) fail 'Invalid service name' ;;
    esac
}

initialize_service() {
    command -v launchctl >/dev/null 2>&1 || fail 'launchctl is not available'
    if [ "$scope" = system ]; then
        domain=system
        directory=/Library/LaunchDaemons
    else
        domain="gui/$(id -u)"
        directory="${HOME:?HOME is required}/Library/LaunchAgents"
    fi
    target=$domain/$service
    plist=$directory/$service.plist
}

run_privileged() {
    if [ "$(id -u)" -eq 0 ]; then
        "$@"
    else
        sudo -- "$@"
    fi
}

run_launchctl() {
    # Read-only queries do not require system privileges or a sudo timestamp.
    if [ "$scope" = user ] || [ "$1" = print ]; then
        launchctl "$@"
    else
        run_privileged launchctl "$@"
    fi
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
    else
        printf 'missing\n'
    fi
}

query_status() {
    loaded=false
    if details=$(run_launchctl print "$target" 2>&1); then
        loaded=true
        case "$details" in
            *'state = running'*) state=active ;;
            *'state = '*) state=inactive ;;
            *) fail 'Unrecognized launchctl print output' ;;
        esac
        return
    fi
    # A failed query is not evidence of absence until its domain is accessible
    # and launchctl explicitly identifies the service as absent.
    case "$details" in
        *'Could not find service'*|*'Could not find specified service'*) ;;
        *) fail "$details" ;;
    esac
    run_launchctl print "$domain" >/dev/null || fail "Cannot query launchd domain: $domain"
    state=$(inspect_plist) || fail "Cannot inspect $plist"
}

require_installed() {
    [ "$state" != missing ] || fail "Service is not installed: $service"
}

unload_service() {
    run_launchctl unload "$plist" || fail "Could not unload $plist"
    query_status
    [ "$loaded" = false ] || fail 'launchctl unload left the service loaded'
}

load_service() {
    file_state=$(inspect_plist) || fail "Cannot inspect $plist"
    [ "$file_state" = inactive ] || fail "Cannot read $plist"
    # Force this manual load without changing the persistent autostart override.
    run_launchctl load -F "$plist" || fail 'launchctl load failed'
    query_status
    attempts=0
    while [ "$loaded" = true ] && [ "$state" != active ] && [ "$attempts" -lt 5 ]; do
        sleep 1
        attempts=$((attempts + 1))
        query_status
    done
    [ "$state" = active ] || fail 'Service did not start; check its plist and core logs'
}

start_service() {
    query_status
    require_installed
    if [ "$state" = active ]; then
        return 0
    fi
    [ "$loaded" = false ] || unload_service
    load_service
}

stop_service() {
    query_status
    [ "$loaded" = false ] || unload_service
}

restart_service() {
    query_status
    require_installed
    [ "$loaded" = false ] || unload_service
    load_service
}

print_status() {
    query_status
    printf '%s\n' "$state"
}

enable_autostart() {
    query_status
    require_installed
    run_launchctl enable "$target"
}

disable_autostart() {
    query_status
    require_installed
    run_launchctl disable "$target"
}

main() {
    set -u
    export LC_ALL=C
    parse_arguments "$@"
    initialize_service

    case "$operation" in
        start) start_service ;;
        stop) stop_service ;;
        restart) restart_service ;;
        status) print_status ;;
        enable-autostart) enable_autostart ;;
        disable-autostart) disable_autostart ;;
    esac
}

main "$@"
