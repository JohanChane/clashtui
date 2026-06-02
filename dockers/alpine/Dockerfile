FROM alpine:3.22

RUN apk add --no-cache \
    openrc \
    sudo \
    bash \
    curl \
    shadow \
    openssl \
    build-base \
    musl-dev \
    openssl-dev \
    openssl-libs-static \
    pkgconf \
    git

RUN echo "root ALL=(ALL) NOPASSWD: ALL" > /etc/sudoers.d/root

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal

ENV PATH="/root/.cargo/bin:${PATH}"

COPY dockers/alpine/entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
CMD ["sleep", "infinity"]
