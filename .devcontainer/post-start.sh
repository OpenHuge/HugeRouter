#!/usr/bin/env bash
set -euo pipefail

WORKSPACE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PERSISTED_CONFIG_PATH="${WORKSPACE_DIR}/.devcontainer/local/codex/config.toml"

mkdir -p "${HOME}/.codex"

if [[ -f "${PERSISTED_CONFIG_PATH}" ]]; then
  ln -sfn "${PERSISTED_CONFIG_PATH}" "${HOME}/.codex/config.toml"
fi
