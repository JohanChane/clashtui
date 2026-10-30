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

check_environment() {
    command -v systemctl >/dev/null 2>&1 || fail 'systemctl is not available'
}

run_privileged() {
    if [ "$(id -u)" -eq 0 ]; then
        "$@"
    else
        sudo -- "$@"
    fi
}

run_systemctl() {
    if [ "$scope" = user ]; then
        systemctl --user "$@"
    elif [ "$1" = show ]; then
        systemctl "$@"
    else
        run_privileged systemctl "$@"
    fi
}

query_status() {
    result=0
    properties=$(run_systemctl show --property=LoadState --property=ActiveState -- "$service") || result=$?
    load_state=
    active_state=
    while IFS='=' read -r key value; do
        case "$key" in
            LoadState) load_state=$value ;;
            ActiveState) active_state=$value ;;
        esac
    done <<EOF_PROPERTIES
$properties
EOF_PROPERTIES

    if [ "$load_state" = not-found ]; then
        state=missing
        return
    fi
    [ "$result" -eq 0 ] || fail "systemctl query failed ($result)"
    [ -n "$load_state" ] || fail 'systemctl returned no LoadState'
    case "$active_state" in
        active|reloading) state=active ;;
        inactive|failed|activating|deactivating|maintenance|refreshing) state=inactive ;;
        *) fail "Unknown systemd state: $active_state" ;;
    esac
}

require_installed() {
    [ "$state" != missing ] || fail "Service is not installed: $service"
}

start_service() {
    query_status
    require_installed
    [ "$state" = active ] || run_systemctl start -- "$service"
}

stop_service() {
    query_status
    if [ "$state" = missing ] || [ "$active_state" = inactive ] || [ "$active_state" = failed ]; then
        return 0
    fi
    run_systemctl stop -- "$service"
}

restart_service() {
    query_status
    require_installed
    run_systemctl restart -- "$service"
}

print_status() {
    query_status
    printf '%s\n' "$state"
}

enable_autostart() {
    query_status
    require_installed
    run_systemctl enable -- "$service"
}

disable_autostart() {
    query_status
    require_installed
    run_systemctl disable -- "$service"
}

main() {
    set -u
    export LC_ALL=C
    parse_arguments "$@"
    check_environment

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
