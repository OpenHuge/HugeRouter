FROM mcr.microsoft.com/devcontainers/javascript-node:24-bookworm

ARG RUST_VERSION=1.94.1
ARG USERNAME=node

RUN apt-get update \
    && export DEBIAN_FRONTEND=noninteractive \
    && apt-get install -y --no-install-recommends \
        build-essential \
        ca-certificates \
        curl \
        git \
        gh \
        jq \
        libssl-dev \
        pkg-config \
        ripgrep \
        unzip \
    && rm -rf /var/lib/apt/lists/*

RUN su "${USERNAME}" -c "curl https://sh.rustup.rs -sSf | sh -s -- -y --profile minimal --default-toolchain ${RUST_VERSION}" \
    && su "${USERNAME}" -c "/home/${USERNAME}/.cargo/bin/rustup component add clippy rustfmt" \
    && corepack enable \
    && mkdir -p "/home/${USERNAME}/.npm-global" "/home/${USERNAME}/.local/share/pnpm" \
    && chown -R "${USERNAME}:${USERNAME}" "/home/${USERNAME}/.npm-global" "/home/${USERNAME}/.local/share/pnpm"

ENV PATH="/home/${USERNAME}/.cargo/bin:/home/${USERNAME}/.local/share/pnpm:/home/${USERNAME}/.npm-global/bin:${PATH}"
