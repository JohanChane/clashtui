FROM alpine:3.22

RUN apk add --no-cache \
    openrc \
    sudo \
    bash \
    curl \
    ca-certificates \
    gcompat

RUN echo "root ALL=(ALL) NOPASSWD: ALL" > /etc/sudoers.d/root

# Create user johan with passwordless sudo
RUN adduser -D -s /bin/bash johan \
    && echo "johan:123" | chpasswd \
    && addgroup johan wheel \
    && echo "johan ALL=(ALL) NOPASSWD: ALL" > /etc/sudoers.d/johan

COPY images/alpine/entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
CMD ["sleep", "infinity"]
