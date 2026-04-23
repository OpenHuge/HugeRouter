#!/usr/bin/env bash
set -euo pipefail

SCENARIO="${1:-gateway-auth-rejection}"
CONTROL_PLANE_BASE_URL="${CONTROL_PLANE_BASE_URL:-http://127.0.0.1:8081}"
GATEWAY_BASE_URL="${GATEWAY_BASE_URL:-http://127.0.0.1:8080}"
TENANT_ID="${TENANT_ID:-tenant_acme}"
RESULTS_FILE="${RESULTS_FILE:-/tmp/huge-router-failure-${SCENARIO}.log}"

record_result() {
  printf '%s\n' "$1" | tee "${RESULTS_FILE}"
}

case "${SCENARIO}" in
  gateway-auth-rejection)
    record_result "$(curl -sS -o /dev/null -w "[failure] gateway unauthenticated status=%{http_code}" \
      -X POST "${GATEWAY_BASE_URL}/v1/responses" \
      -H "content-type: application/json" \
      --data '{"model":"reasoning-fast","input":[{"role":"user","content":[{"type":"input_text","text":"auth failure probe"}]}]}')"
    ;;
  control-plane-missing-tenant)
    record_result "$(curl -sS -o /dev/null -w "[failure] usage summary missing tenant status=%{http_code}" \
      "${CONTROL_PLANE_BASE_URL}/v1/usage/summary")"
    ;;
  control-plane-breakdown-invalid-group)
    record_result "$(curl -sS -o /dev/null -w "[failure] usage breakdown invalid group status=%{http_code}" \
      "${CONTROL_PLANE_BASE_URL}/v1/usage/breakdown?tenant_id=${TENANT_ID}&group_by=invalid")"
    ;;
  retry-storm)
    seq 20 | xargs -I{} -P 10 sh -c \
      "curl -sS -o /dev/null -w 'retry-storm status=%{http_code}\n' '${CONTROL_PLANE_BASE_URL}/v1/usage/breakdown?tenant_id=${TENANT_ID}&group_by=invalid'" \
      | tee "${RESULTS_FILE}"
    ;;
  *)
    echo "usage: $0 [gateway-auth-rejection|control-plane-missing-tenant|control-plane-breakdown-invalid-group|retry-storm]" >&2
    exit 1
    ;;
esac
