#!/usr/bin/env bash
set -euo pipefail

mkdir -p "${NPM_CONFIG_PREFIX:-${HOME}/.npm-global}"
npm i -g @openai/codex@latest
