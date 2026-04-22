#!/usr/bin/env bash
set -euo pipefail

readonly STACK_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly STACK_REPO_ROOT="$(cd "${STACK_SCRIPT_DIR}/../../.." && pwd)"
readonly STACK_COMPOSE_FILE="${STACK_REPO_ROOT}/infra/docker/compose.yaml"

resolve_stack_mode() {
  local mode="${1:-${HUGE_ROUTER_STACK_MODE:-core}}"

  case "${mode}" in
    core | full | observability)
      printf '%s\n' "${mode}"
      ;;
    *)
      echo "Unsupported stack mode: ${mode}. Expected one of: core, full, observability." >&2
      return 1
      ;;
  esac
}

stack_services_for_mode() {
  case "$1" in
    core)
      printf '%s\n' postgres redis nats
      ;;
    observability)
      printf '%s\n' otel-collector prometheus grafana
      ;;
    full)
      printf '%s\n' postgres redis nats otel-collector prometheus grafana
      ;;
    *)
      echo "Unsupported stack mode: $1" >&2
      return 1
      ;;
  esac
}

stack_compose() {
  local mode="$1"
  shift

  local -a compose_args=(-f "${STACK_COMPOSE_FILE}")

  if [[ "${mode}" != "core" ]]; then
    compose_args+=(--profile observability)
  fi

  docker compose "${compose_args[@]}" "$@"
}
