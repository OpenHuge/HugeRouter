#!/usr/bin/env bash
set -euo pipefail

WORKSPACE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

link_dir() {
  local source_dir="$1"
  local target_dir="$2"

  mkdir -p "$(dirname "${target_dir}")" "${source_dir}"
  rm -rf "${target_dir}"
  ln -s "${source_dir}" "${target_dir}"
}

mkdir -p "${HOME}/.codex"

PERSISTED_CODEX_DIR="${WORKSPACE_DIR}/.devcontainer/local/codex"
PERSISTED_CODEX_PATH="${PERSISTED_CODEX_DIR}/config.toml"

mkdir -p "${PERSISTED_CODEX_DIR}"

if [[ ! -f "${PERSISTED_CODEX_PATH}" ]]; then
  install -m 600 "${WORKSPACE_DIR}/.devcontainer/codex-config.toml" "${PERSISTED_CODEX_PATH}"
fi

ln -sfn "${PERSISTED_CODEX_PATH}" "${HOME}/.codex/config.toml"

PERSISTED_GH_DIR="${WORKSPACE_DIR}/.devcontainer/local/gh"
link_dir "${PERSISTED_GH_DIR}" "${HOME}/.config/gh"
