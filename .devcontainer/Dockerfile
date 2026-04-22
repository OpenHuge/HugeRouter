FROM mcr.microsoft.com/devcontainers/base:bookworm

ARG NODE_VERSION=24.11.1
ARG PNPM_VERSION=10.33.0
ARG RUST_VERSION=1.95.0
ARG USERNAME=vscode

RUN apt-get update \
    && export DEBIAN_FRONTEND=noninteractive \
    && apt-get install -y --no-install-recommends \
        build-essential \
        ca-certificates \
        curl \
        git \
        jq \
        libssl-dev \
        pkg-config \
        ripgrep \
        unzip \
    && rm -rf /var/lib/apt/lists/*

RUN ARCH="$(dpkg --print-architecture)" \
    && case "${ARCH}" in \
        amd64) NODE_ARCH="x64" ;; \
        arm64) NODE_ARCH="arm64" ;; \
        *) echo "Unsupported architecture: ${ARCH}" && exit 1 ;; \
      esac \
    && curl -fsSL "https://nodejs.org/dist/v${NODE_VERSION}/node-v${NODE_VERSION}-linux-${NODE_ARCH}.tar.xz" -o /tmp/node.tar.xz \
    && tar -xJf /tmp/node.tar.xz -C /usr/local --strip-components=1 --no-same-owner \
    && rm /tmp/node.tar.xz \
    && corepack enable \
    && corepack prepare "pnpm@${PNPM_VERSION}" --activate

RUN su "${USERNAME}" -c "curl https://sh.rustup.rs -sSf | sh -s -- -y --profile minimal --default-toolchain ${RUST_VERSION}" \
    && su "${USERNAME}" -c "/home/${USERNAME}/.cargo/bin/rustup component add clippy rustfmt"

RUN mkdir -p "/home/${USERNAME}/.npm-global" \
    && chown -R "${USERNAME}:${USERNAME}" "/home/${USERNAME}/.npm-global"

ENV PATH="/home/${USERNAME}/.cargo/bin:/home/${USERNAME}/.local/share/pnpm:/home/${USERNAME}/.npm-global/bin:${PATH}"
