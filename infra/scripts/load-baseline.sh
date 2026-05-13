#!/usr/bin/env bash
set -euo pipefail

MODE="${1:-usage-summary}"
CONCURRENCY="${CONCURRENCY:-10}"
ITERATIONS="${ITERATIONS:-50}"
SOAK_SECONDS="${SOAK_SECONDS:-60}"
RESULTS_FILE="${RESULTS_FILE:-/tmp/huge-router-load-${MODE}.json}"
TENANT_ID="${TENANT_ID:-tenant_acme}"
CONTROL_PLANE_BASE_URL="${CONTROL_PLANE_BASE_URL:-http://127.0.0.1:8081}"
GATEWAY_BASE_URL="${GATEWAY_BASE_URL:-http://127.0.0.1:8080}"
GATEWAY_API_KEY="${GATEWAY_API_KEY:-}"

run_parallel() {
  local request_cmd="$1"
  seq "${ITERATIONS}" | xargs -I{} -P "${CONCURRENCY}" sh -c "${request_cmd}" >/dev/null
}

record_result() {
  local started_at="$1"
  local finished_at="$2"
  cat >"${RESULTS_FILE}" <<EOF
{
  "mode": "${MODE}",
  "concurrency": ${CONCURRENCY},
  "iterations": ${ITERATIONS},
  "soak_seconds": ${SOAK_SECONDS},
  "started_at": "${started_at}",
  "finished_at": "${finished_at}"
}
EOF
}

run_soak() {
  local request_cmd="$1"
  local deadline=$(( $(date +%s) + SOAK_SECONDS ))
  while [[ "$(date +%s)" -lt "${deadline}" ]]; do
    seq "${CONCURRENCY}" | xargs -I{} -P "${CONCURRENCY}" sh -c "${request_cmd}" >/dev/null
  done
}

started_at="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
case "${MODE}" in
  usage-summary)
    echo "[load] running usage summary baseline against ${CONTROL_PLANE_BASE_URL}"
    run_parallel "curl -fsS '${CONTROL_PLANE_BASE_URL}/v1/usage/summary?tenant_id=${TENANT_ID}'"
    ;;
  usage-breakdown)
    echo "[load] running usage breakdown baseline against ${CONTROL_PLANE_BASE_URL}"
    run_parallel "curl -fsS '${CONTROL_PLANE_BASE_URL}/v1/usage/breakdown?tenant_id=${TENANT_ID}&group_by=provider'"
    ;;
  gateway-responses)
    if [[ -z "${GATEWAY_API_KEY}" ]]; then
      echo "[load] GATEWAY_API_KEY is required for gateway-responses mode" >&2
      exit 1
    fi
    echo "[load] running gateway responses baseline against ${GATEWAY_BASE_URL}"
    run_parallel "curl -fsS -X POST '${GATEWAY_BASE_URL}/v1/responses' -H 'content-type: application/json' -H 'authorization: Bearer ${GATEWAY_API_KEY}' --data '{\"model\":\"reasoning-fast\",\"input\":[{\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"baseline ping\"}]}]}'"
    ;;
  soak-usage-summary)
    echo "[load] running usage summary soak against ${CONTROL_PLANE_BASE_URL}"
    run_soak "curl -fsS '${CONTROL_PLANE_BASE_URL}/v1/usage/summary?tenant_id=${TENANT_ID}'"
    ;;
  soak-usage-breakdown)
    echo "[load] running usage breakdown soak against ${CONTROL_PLANE_BASE_URL}"
    run_soak "curl -fsS '${CONTROL_PLANE_BASE_URL}/v1/usage/breakdown?tenant_id=${TENANT_ID}&group_by=provider'"
    ;;
  *)
    echo "usage: $0 [usage-summary|usage-breakdown|gateway-responses|soak-usage-summary|soak-usage-breakdown]" >&2
    exit 1
    ;;
esac

finished_at="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
record_result "${started_at}" "${finished_at}"
echo "[load] wrote baseline result to ${RESULTS_FILE}"
echo "[load] completed mode=${MODE} concurrency=${CONCURRENCY}"
