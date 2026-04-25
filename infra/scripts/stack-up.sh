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
  while IFS= read -r service; do
    services+=("${service}")
  done < <(stack_services_for_mode observability)
fi

if ((${#services[@]} > 0)); then
  stack_compose "${mode}" up -d --remove-orphans "${services[@]}"
  exit 0
fi

case "${mode}" in
  core | observability)
    stack_compose "${mode}" up -d --remove-orphans "${services[@]}"
    ;;
  runtime | full)
    stack_compose "${mode}" up -d --remove-orphans postgres redis nats

    if [[ "${mode}" == "full" ]]; then
      stack_compose "${mode}" up -d --remove-orphans otel-collector alertmanager prometheus grafana
    fi

    if [[ "${HUGE_ROUTER_STACK_SKIP_INIT:-false}" != "true" ]]; then
      "${SCRIPT_DIR}/migrate.sh" "${mode}"
      "${SCRIPT_DIR}/bootstrap.sh" "${mode}"
    fi

    stack_compose "${mode}" up -d --remove-orphans \
      control-plane-api \
      ledger-worker \
      route-receipt-worker \
      routing-worker \
      edge-probe \
      audit-worker \
      notification-worker
    ;;
esac
