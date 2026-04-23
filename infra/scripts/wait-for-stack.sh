#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/stack-common.sh"

mode="$(resolve_stack_mode "${1:-}")"
if (($# > 0)); then
  shift
fi

timeout_seconds="${1:-${HUGE_ROUTER_STACK_TIMEOUT_SECONDS:-180}}"

declare -a services=()
while IFS= read -r service; do
  services+=("${service}")
done < <(stack_services_for_mode "${mode}")
deadline=$((SECONDS + timeout_seconds))

while ((SECONDS < deadline)); do
  all_ready=true

  for service in "${services[@]}"; do
    container_id="$(stack_compose "${mode}" ps -q "${service}")"
    container_id="${container_id//$'\n'/}"

    if [[ -z "${container_id}" ]]; then
      all_ready=false
      continue
    fi

    status="$(docker inspect --format '{{if .State.Health}}{{.State.Health.Status}}{{else}}{{.State.Status}}{{end}}' "${container_id}" 2>/dev/null || true)"

    if [[ "${status}" != "healthy" && "${status}" != "running" ]]; then
      all_ready=false
    fi
  done

  if [[ "${all_ready}" == "true" ]]; then
    printf 'Stack mode %s is ready.\n' "${mode}"
    exit 0
  fi

  sleep 2
done

echo "Timed out waiting for stack mode ${mode} after ${timeout_seconds}s." >&2

for service in "${services[@]}"; do
  container_id="$(stack_compose "${mode}" ps -q "${service}")"
  container_id="${container_id//$'\n'/}"

  if [[ -z "${container_id}" ]]; then
    echo "- ${service}: not started" >&2
    continue
  fi

  status="$(docker inspect --format '{{if .State.Health}}{{.State.Health.Status}}{{else}}{{.State.Status}}{{end}}' "${container_id}" 2>/dev/null || true)"
  echo "- ${service}: ${status:-unknown}" >&2
done

exit 1
