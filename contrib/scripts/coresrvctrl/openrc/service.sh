#!/bin/sh
# Usage: service.sh OP system|user SERVICE
set -u
export LC_ALL=C RC_NOCOLOR=YES
fail() { printf '%s\n' "$*" >&2; exit 1; }
[ "$#" -eq 3 ] || fail 'Expected OP SCOPE SERVICE'
op=$1 scope=$2 service=$3
case "$op" in start|stop|restart|status|enable-autostart|disable-autostart) ;; *) fail "Unknown operation: $op";; esac
case "$scope" in system|user) ;; *) fail "Unknown scope: $scope";; esac
case "$service" in ''|-*|*[!a-zA-Z0-9_.@:-]*) fail 'Invalid service name';; esac
command -v rc-service >/dev/null 2>&1 || fail 'rc-service is not available'
if [ "$scope" = user ]; then
    [ -n "${XDG_RUNTIME_DIR:-}" ] && [ -d "$XDG_RUNTIME_DIR" ] && [ -x "$XDG_RUNTIME_DIR" ] || fail 'OpenRC user services require an accessible XDG_RUNTIME_DIR'
    help=$(rc-service --help 2>&1) || fail 'Cannot inspect rc-service capabilities'
    case "$help" in *--user*) ;; *) fail 'This OpenRC version does not support user services';; esac
fi
privileged() {
    if [ "$(id -u)" -eq 0 ]; then "$@"; else sudo -- "$@"; fi
}
svc() {
    if [ "$scope" = user ]; then
        rc-service --user "$@"
    elif [ "$1" = --exists ] || [ "${2:-}" = status ]; then
        rc-service "$@"
    else
        privileged rc-service "$@"
    fi
}
update() {
    command -v rc-update >/dev/null 2>&1 || fail 'rc-update is not available'
    if [ "$scope" = user ]; then
        rc-update --user "$@"
    elif [ "$1" = show ]; then
        rc-update "$@"
    else
        privileged rc-update "$@"
    fi
}
query() {
    stopped=false
    result=0
    # Redirect the external command only, so shell xtrace is not mistaken for
    # an existence-query diagnostic when rc-service returns an empty exit 1.
    if [ "$scope" = user ]; then set -- rc-service --user; else set -- rc-service; fi
    details=$("$@" --exists "$service" 2>&1) || result=$?
    if [ "$result" -ne 0 ]; then
        [ "$result" -eq 1 ] && [ -z "$details" ] || fail "OpenRC existence query failed: $details"
        # Do not infer absence from a service directory we cannot inspect.
        if [ "$scope" = user ]; then
            set -- /etc/user/init.d "${XDG_CONFIG_HOME:-${HOME:?HOME is required}/.config}/rc/init.d"
        else set -- /etc/init.d /usr/local/etc/init.d; fi
        for directory do
            [ ! -d "$directory" ] || { [ -r "$directory" ] && [ -x "$directory" ]; } || fail "Cannot inspect $directory"
        done
        state=missing
        stopped=true
        return
    fi
    result=0
    details=$(svc "$service" status 2>&1) || result=$?
    case "$result:$details" in
        0:*'status: started'*) state=active;;
        3:*'status: stopped'*) state=inactive; stopped=true;;
        4:*'status: stopping'*|8:*'status: starting'*|16:*'status: inactive'*|32:*'status: crashed'*) state=inactive;;
        *) fail "OpenRC status query failed ($result): $details";;
    esac
}
query
case "$op" in
    status) printf '%s\n' "$state";;
    stop) [ "$stopped" = true ] || svc "$service" stop;;
    start)
        [ "$state" != missing ] || fail "Service is not installed: $service"
        [ "$state" = active ] || svc "$service" start;;
    restart)
        [ "$state" != missing ] || fail "Service is not installed: $service"
        svc "$service" restart;;
    enable-autostart|disable-autostart)
        [ "$state" != missing ] || fail "Service is not installed: $service"
        if [ "$op" = enable-autostart ]; then update add "$service" default
        else
            # Deleting an absent membership is an error in rc-update itself.
            membership=$(update show default) || fail 'Cannot query OpenRC runlevel membership'
            enabled=false
            while read -r name separator runlevel; do
                if [ "$name" = "$service" ] && [ "$separator" = '|' ] && [ "$runlevel" = default ]; then enabled=true; fi
            done <<EOF
$membership
EOF
            [ "$enabled" = false ] || update del "$service" default
        fi;;
esac
