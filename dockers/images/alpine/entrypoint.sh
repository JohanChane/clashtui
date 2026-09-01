#!/bin/sh
set -e

if [ ! -f /run/openrc/softlevel ]; then
    mkdir -p /run/openrc
    touch /run/openrc/softlevel
fi

for svc in /etc/init.d/*; do
    [ -x "$svc" ] && rc-update add "$(basename "$svc")" default 2>/dev/null || true
done

echo "=== Alpine OpenRC + clashtui Dev Container ==="
echo "Project: /workspace/clashtui"
echo "OpenRC ready. Use 'rc-service <name> start' etc."
echo ""

exec "$@"
