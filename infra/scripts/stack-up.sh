#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/stack-common.sh"

mode="$(resolve_stack_mode "${1:-}")"
if (($# > 0)); then
  shift
fi

declare -a services=()
if (($# > 0)); then
  services=("$@")
elif [[ "${mode}" == "observability" ]]; then
  mapfile -t services < <(stack_services_for_mode observability)
fi

stack_compose "${mode}" up -d --remove-orphans "${services[@]}"
