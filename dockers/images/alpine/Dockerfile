FROM alpine:3.22

RUN apk add --no-cache \
    openrc \
    sudo \
    bash \
    curl \
    ca-certificates

RUN echo "root ALL=(ALL) NOPASSWD: ALL" > /etc/sudoers.d/root

COPY images/alpine/entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
CMD ["sleep", "infinity"]
