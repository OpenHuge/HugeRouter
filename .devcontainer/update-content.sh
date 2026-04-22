#!/usr/bin/env bash
set -euo pipefail

cd /workspaces/HugeRouter

corepack enable
corepack prepare pnpm@10.33.0 --activate
pnpm verify:toolchain
pnpm install --frozen-lockfile
cargo fetch

if ! command -v just >/dev/null 2>&1; then
  cargo install just --locked
fi
