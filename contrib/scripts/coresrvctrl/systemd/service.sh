#!/bin/sh
# Usage: service.sh OP system|user SERVICE
set -u
export LC_ALL=C
fail() { printf '%s\n' "$*" >&2; exit 1; }
[ "$#" -eq 3 ] || fail 'Expected OP SCOPE SERVICE'
op=$1 scope=$2 service=$3
case "$op" in start|stop|restart|status|enable-autostart|disable-autostart) ;; *) fail "Unknown operation: $op";; esac
case "$scope" in system|user) ;; *) fail "Unknown scope: $scope";; esac
case "$service" in ''|-*|*[!a-zA-Z0-9_.@:-]*) fail 'Invalid service name';; esac
command -v systemctl >/dev/null 2>&1 || fail 'systemctl is not available'
ctl() {
    if [ "$scope" = user ]; then systemctl --user "$@"; else systemctl "$@"; fi
}
query() {
    result=0
    properties=$(ctl show --property=LoadState --property=ActiveState -- "$service") || result=$?
    load= active=
    while IFS='=' read -r key value; do
        case "$key" in LoadState) load=$value;; ActiveState) active=$value;; esac
    done <<EOF
$properties
EOF
    if [ "$load" = not-found ]; then state=missing; return; fi
    [ "$result" -eq 0 ] || fail "systemctl query failed ($result)"
    [ -n "$load" ] || fail 'systemctl returned no LoadState'
    case "$active" in
        active|reloading) state=active;;
        inactive|failed|activating|deactivating|maintenance|refreshing) state=inactive;;
        *) fail "Unknown systemd state: $active";;
    esac
}
query
case "$op" in
    status) printf '%s\n' "$state";;
    stop) [ "$active" = inactive ] || [ "$active" = failed ] || [ "$state" = missing ] || ctl stop -- "$service";;
    start)
        [ "$state" != missing ] || fail "Service is not installed: $service"
        [ "$state" = active ] || ctl start -- "$service";;
    restart)
        [ "$state" != missing ] || fail "Service is not installed: $service"
        ctl restart -- "$service";;
    enable-autostart|disable-autostart)
        [ "$state" != missing ] || fail "Service is not installed: $service"
        case "$op" in enable-autostart) ctl enable -- "$service";; *) ctl disable -- "$service";; esac;;
esac
