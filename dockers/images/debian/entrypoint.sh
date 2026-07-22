#!/bin/bash
set -e

# Ensure a writable machine-id for systemd
if [ ! -s /etc/machine-id ]; then
	echo "init" >/etc/machine-id
fi

# Set up hostname so systemd doesn't choke
hostnamectl set-hostname clashtui-deb-dev 2>/dev/null || hostname clashtui-deb-dev

echo "=== Debian 13 (Trixie) systemd + clashtui Dev Container ==="
echo "Project: /home/johan/workspace/clashtui"
echo "systemd PID 1 ready."
echo ""

exec "$@"
