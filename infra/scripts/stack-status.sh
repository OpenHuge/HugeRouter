#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/stack-common.sh"

mode="$(resolve_stack_mode "${1:-}")"
declare -a services=()
while IFS= read -r service; do
  services+=("${service}")
done < <(stack_services_for_mode "${mode}")

stack_compose "${mode}" ps "${services[@]}"
printf '\n'

for service in "${services[@]}"; do
  container_id="$(stack_compose "${mode}" ps -q "${service}")"
  container_id="${container_id//$'\n'/}"

  if [[ -z "${container_id}" ]]; then
    printf '%s: not started\n' "${service}"
    continue
  fi

  status="$(docker inspect --format '{{if .State.Health}}{{.State.Health.Status}}{{else}}{{.State.Status}}{{end}}' "${container_id}" 2>/dev/null || true)"
  printf '%s: %s\n' "${service}" "${status:-unknown}"
done
