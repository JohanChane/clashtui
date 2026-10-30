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
    command -v rc-service >/dev/null 2>&1 || fail 'rc-service is not available'
    if [ "$scope" = user ]; then
        [ -n "${XDG_RUNTIME_DIR:-}" ] && [ -d "$XDG_RUNTIME_DIR" ] && [ -x "$XDG_RUNTIME_DIR" ] || fail 'OpenRC user services require an accessible XDG_RUNTIME_DIR'
        help=$(rc-service --help 2>&1) || fail 'Cannot inspect rc-service capabilities'
        case "$help" in
            *--user*) ;;
            *) fail 'This OpenRC version does not support user services' ;;
        esac
    fi
}

run_privileged() {
    if [ "$(id -u)" -eq 0 ]; then
        "$@"
    else
        sudo -- "$@"
    fi
}

run_service_command() {
    if [ "$scope" = user ]; then
        rc-service --user "$@"
    elif [ "$1" = --exists ] || [ "${2:-}" = status ]; then
        rc-service "$@"
    else
        run_privileged rc-service "$@"
    fi
}

run_update_command() {
    command -v rc-update >/dev/null 2>&1 || fail 'rc-update is not available'
    if [ "$scope" = user ]; then
        rc-update --user "$@"
    elif [ "$1" = show ]; then
        rc-update "$@"
    else
        run_privileged rc-update "$@"
    fi
}

check_service_directories() {
    # Do not infer absence from a service directory we cannot inspect.
    if [ "$scope" = user ]; then
        set -- /etc/user/init.d "${XDG_CONFIG_HOME:-${HOME:?HOME is required}/.config}/rc/init.d"
    else
        set -- /etc/init.d /usr/local/etc/init.d
    fi
    for directory do
        [ ! -d "$directory" ] || { [ -r "$directory" ] && [ -x "$directory" ]; } || fail "Cannot inspect $directory"
    done
}

query_status() {
    stopped=false
    result=0
    # Redirect the external command only, so shell xtrace is not mistaken for
    # an existence-query diagnostic when rc-service returns an empty exit 1.
    if [ "$scope" = user ]; then
        set -- rc-service --user
    else
        set -- rc-service
    fi
    details=$("$@" --exists "$service" 2>&1) || result=$?
    if [ "$result" -ne 0 ]; then
        [ "$result" -eq 1 ] && [ -z "$details" ] || fail "OpenRC existence query failed: $details"
        check_service_directories
        state=missing
        stopped=true
        return
    fi

    result=0
    details=$(run_service_command "$service" status 2>&1) || result=$?
    case "$result:$details" in
        0:*'status: started'*) state=active ;;
        3:*'status: stopped'*) state=inactive; stopped=true ;;
        4:*'status: stopping'*|8:*'status: starting'*|16:*'status: inactive'*|32:*'status: crashed'*) state=inactive ;;
        *) fail "OpenRC status query failed ($result): $details" ;;
    esac
}

require_installed() {
    [ "$state" != missing ] || fail "Service is not installed: $service"
}

start_service() {
    query_status
    require_installed
    [ "$state" = active ] || run_service_command "$service" start
}

stop_service() {
    query_status
    [ "$stopped" = true ] || run_service_command "$service" stop
}

restart_service() {
    query_status
    require_installed
    run_service_command "$service" restart
}

print_status() {
    query_status
    printf '%s\n' "$state"
}

enable_autostart() {
    query_status
    require_installed
    run_update_command add "$service" default
}

disable_autostart() {
    query_status
    require_installed
    # Deleting an absent membership is an error in rc-update itself.
    membership=$(run_update_command show default) || fail 'Cannot query OpenRC runlevel membership'
    enabled=false
    while read -r name separator runlevel; do
        if [ "$name" = "$service" ] && [ "$separator" = '|' ] && [ "$runlevel" = default ]; then
            enabled=true
        fi
    done <<EOF_MEMBERSHIP
$membership
EOF_MEMBERSHIP
    [ "$enabled" = false ] || run_update_command del "$service" default
}

main() {
    set -u
    export LC_ALL=C RC_NOCOLOR=YES
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
