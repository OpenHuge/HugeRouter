#!/usr/bin/env bash
set -euo pipefail

cd /workspaces/HugeRouter

CARGO_HOME_DIR="${CARGO_HOME:-/home/node/.cargo}"
PNPM_HOME_DIR="${PNPM_HOME:-/home/node/.local/share/pnpm}"

# Named volumes mounted for non-root users can come in owned by root on creation.
# Dev Containers docs recommend chowning such mounts before using them.
if command -v sudo >/dev/null 2>&1 && sudo -n true >/dev/null 2>&1; then
  sudo mkdir -p \
    "${CARGO_HOME_DIR}/git" \
    "${CARGO_HOME_DIR}/registry" \
    "${PNPM_HOME_DIR}/store"
  sudo chown -R "$(id -u):$(id -g)" "${CARGO_HOME_DIR}" "${PNPM_HOME_DIR}"
fi

for required_dir in "${CARGO_HOME_DIR}" "${CARGO_HOME_DIR}/registry" "${PNPM_HOME_DIR}" "${PNPM_HOME_DIR}/store"; do
  if [[ ! -w "${required_dir}" ]]; then
    echo "Required devcontainer cache directory is not writable: ${required_dir}" >&2
    echo "Ensure mounted named volumes are owned by the remote user before running update-content.sh." >&2
    exit 1
  fi
done

corepack enable
corepack prepare pnpm@10.33.0 --activate
pnpm verify:toolchain
pnpm install --frozen-lockfile
cargo fetch

if ! command -v just >/dev/null 2>&1; then
  cargo install just --locked
fi
