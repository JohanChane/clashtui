FROM debian:trixie

# Avoid interactive prompts during package install
ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y --no-install-recommends \
    systemd \
    systemd-sysv \
    dbus \
    sudo \
    bash \
    curl \
    ca-certificates \
    fzf \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Allow root to sudo without password
RUN echo "root ALL=(ALL) NOPASSWD: ALL" > /etc/sudoers.d/root

# Create user johan with passwordless sudo
RUN useradd -m -s /bin/bash johan \
    && echo "johan:123" | chpasswd \
    && usermod -aG sudo johan \
    && echo "johan ALL=(ALL) NOPASSWD: ALL" > /etc/sudoers.d/johan

# Mask irrelevant systemd units that would fail in a container
RUN systemctl mask \
    dev-ttyS*.device \
    getty.target \
    getty@.service \
    console-getty.service \
    systemd-udevd.service \
    systemd-udevd-kernel.socket \
    systemd-udevd-control.socket \
    systemd-modules-load.service \
    systemd-ask-password-console.path \
    systemd-ask-password-wall.path \
    systemd-logind.service \
    systemd-random-seed.service \
    systemd-update-utmp.service \
    systemd-resolved.service \
    ; :

COPY images/debian/entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

# Writable machine-id for systemd
RUN : > /etc/machine-id

ENTRYPOINT ["/entrypoint.sh"]
CMD ["/sbin/init"]
