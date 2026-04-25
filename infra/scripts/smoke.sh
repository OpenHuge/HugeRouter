#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
RUN_ID="smoke-$(date +%s)"
LOG_ROOT="/tmp/huge-router-smoke-${RUN_ID}"
mkdir -p "${LOG_ROOT}"

require() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "[smoke] missing required command: $1" >&2
    exit 1
  fi
}

log() {
  echo "[smoke] $*"
}

CONTROL_PLANE_COOKIE_JAR=""

cleanup() {
  local code=$?
  if [[ "${DOCKER_STACK_STARTED:-false}" == "true" ]]; then
    docker compose "${COMPOSE_ARGS[@]}" down -v --remove-orphans >/dev/null 2>&1 || true
  fi

  if [[ "${code}" -ne 0 ]]; then
    log "smoke failed; inspect logs in ${LOG_ROOT}"
  fi
  exit "${code}"
}

trap cleanup EXIT

wait_for_http() {
  local url="$1"
  local timeout="${2:-90}"
  local label="$3"
  local attempt=0
  while ((attempt < timeout)); do
    if curl -fsS "${url}" >/dev/null 2>&1; then
      return 0
    fi
    attempt=$((attempt + 1))
    sleep 1
  done
  log "timeout waiting for ${label}: ${url}"
  return 1
}

wait_for_compose_service_state() {
  local service="$1"
  local expected_state="${2:-running}"
  local timeout="${3:-90}"
  local attempt=0

  while ((attempt < timeout)); do
    local container_id
    local state

    container_id="$(docker compose "${COMPOSE_ARGS[@]}" ps -q "${service}")"
    container_id="${container_id//$'\n'/}"

    if [[ -n "${container_id}" ]]; then
      state="$(docker inspect --format '{{if .State.Health}}{{.State.Health.Status}}{{else}}{{.State.Status}}{{end}}' "${container_id}" 2>/dev/null || true)"
      if [[ "${state}" == "${expected_state}" ]]; then
        return 0
      fi
    fi

    if [[ "${expected_state}" == "running" && "${state:-}" == "healthy" ]]; then
      return 0
    fi

    attempt=$((attempt + 1))
    sleep 1
  done

  log "timeout waiting for ${service} to reach state ${expected_state}"
  return 1
}

query_ledger_count() {
  docker compose "${COMPOSE_ARGS[@]}" exec -T \
    -e PGPASSWORD="${POSTGRES_PASSWORD}" postgres \
    psql -h 127.0.0.1 -U "${POSTGRES_USER}" -d "${POSTGRES_DB}" -tAc \
    "SELECT COUNT(*) FROM ledger_entries;"
}

query_route_receipt_count() {
  docker compose "${COMPOSE_ARGS[@]}" exec -T \
    -e PGPASSWORD="${POSTGRES_PASSWORD}" postgres \
    psql -h 127.0.0.1 -U "${POSTGRES_USER}" -d "${POSTGRES_DB}" -tAc \
    "SELECT COUNT(*) FROM route_receipts;"
}

query_route_receipt_diagnostics_count() {
  docker compose "${COMPOSE_ARGS[@]}" exec -T \
    -e PGPASSWORD="${POSTGRES_PASSWORD}" postgres \
    psql -h 127.0.0.1 -U "${POSTGRES_USER}" -d "${POSTGRES_DB}" -tAc \
    "SELECT COUNT(*) FROM route_receipt_diagnostics;"
}

create_provider_resource() {
  local provider_id="$1"
  local provider_resource_id="$2"
  local display_name="$3"
  local endpoint_base_url="$4"
  local payload
  payload="$(jq -n \
    --arg provider_resource_id "${provider_resource_id}" \
    --arg tenant_id "tenant_acme" \
    --arg project_id "proj_core" \
    --arg provider_id "${provider_id}" \
    --arg display_name "${display_name}" \
    --arg endpoint_base_url "${endpoint_base_url}" \
    '{
      provider_resource_id: $provider_resource_id,
      tenant_id: $tenant_id,
      project_id: $project_id,
      provider_id: $provider_id,
      name: $display_name,
      status: "active",
      provenance_class: "official_api",
      credential_owner_type: "platform",
      deployment_scope: "shared",
      region: "global",
      endpoint_base_url: $endpoint_base_url,
      auth_kind: "api_key",
      health_state: "healthy",
      capabilities: {
        supports_streaming: false,
        supports_tool_calling: false,
        supports_json_mode: true
      },
      version: 1,
      created_at: "2026-04-22T00:00:00Z",
      updated_at: "2026-04-22T00:00:00Z"
    }')"

  curl -fsS -X POST "${CONTROL_PLANE_BASE_URL}/v1/provider-resources" \
    -H "content-type: application/json" \
    --data "${payload}" >/dev/null
}

create_route_policy() {
  local route_policy_id="$1"
  local protocol_family="$2"
  local model_alias="$3"
  local display_name="$4"
  local payload
  payload="$(jq -n \
    --arg route_policy_id "${route_policy_id}" \
    --arg tenant_id "tenant_acme" \
    --arg display_name "${display_name}" \
    --arg protocol_family "${protocol_family}" \
    --arg model_alias "${model_alias}" \
    '{
      route_policy_id: $route_policy_id,
      tenant_id: $tenant_id,
      display_name: $display_name,
      protocol_family: $protocol_family,
      model_alias: $model_alias,
      required_capabilities: ["chat_completions"],
      preferred_regions: ["global"],
      version: 1,
      created_at: "2026-04-22T00:00:00Z",
      updated_at: "2026-04-22T00:00:00Z"
    }')"

  curl -fsS -X POST "${CONTROL_PLANE_BASE_URL}/v1/route-policies" \
    -H "content-type: application/json" \
    --data "${payload}" >/dev/null
}

create_and_activate_snapshot() {
  local config_snapshot_id="$1"
  local route_policy_id="$2"
  local provider_resource_id="$3"
  local revision="$4"
  local payload
  payload="$(jq -n \
    --arg config_snapshot_id "${config_snapshot_id}" \
    --arg tenant_id "tenant_acme" \
    --arg project_id "proj_core" \
    --arg provider_resource_id "${provider_resource_id}" \
    --arg route_policy_id "${route_policy_id}" \
    --argjson revision "${revision}" \
    '{
      config_snapshot_id: $config_snapshot_id,
      tenant_id: $tenant_id,
      project_id: $project_id,
      revision: $revision,
      status: "draft",
      provider_resource_ids: [$provider_resource_id],
      route_policy_id: $route_policy_id,
      budget_policy_id: "budgetpol_default"
    }')"

  curl -fsS -X POST "${CONTROL_PLANE_BASE_URL}/v1/config-snapshots" \
    -H "content-type: application/json" \
    --data "${payload}" >/dev/null
  curl -fsS -X POST "${CONTROL_PLANE_BASE_URL}/v1/config-snapshots/${config_snapshot_id}/activate" >/dev/null
}

login_platform_admin() {
  local start_payload
  local flow_file="${LOG_ROOT}/auth_email_start.json"
  local complete_file="${LOG_ROOT}/auth_email_complete.json"
  CONTROL_PLANE_COOKIE_JAR="${LOG_ROOT}/control-plane.cookies.txt"

  start_payload="$(jq -n \
    --arg email "ops@huge-router.dev" \
    --arg workspace_slug "platform-admin" \
    '{email:$email,workspaceSlug:$workspace_slug}')"

  curl -fsS -o "${flow_file}" \
    -X POST "${CONTROL_PLANE_BASE_URL}/api/control-plane/auth/email/start" \
    -H "content-type: application/json" \
    --data "${start_payload}"

  local flow_id
  flow_id="$(jq -r '.flowId // empty' "${flow_file}")"
  if [[ -z "${flow_id}" ]]; then
    log "failed to obtain platform-admin auth flow id"
    log "response: $(cat "${flow_file}")"
    exit 1
  fi

  curl -fsS -o "${complete_file}" -c "${CONTROL_PLANE_COOKIE_JAR}" \
    -X POST "${CONTROL_PLANE_BASE_URL}/api/control-plane/auth/email/complete" \
    -H "content-type: application/json" \
    --data "{\"code\":\"111111\",\"flowId\":\"${flow_id}\"}"

  if ! grep -q 'huge_router_session' "${CONTROL_PLANE_COOKIE_JAR}" 2>/dev/null; then
    log "platform-admin session cookie was not issued"
    log "response: $(cat "${complete_file}")"
    exit 1
  fi
}

wait_for_ledger_increment() {
  local before_count="$1"
  local after_count
  local attempt=0
  while :; do
    after_count="$(query_ledger_count | tr -d '[:space:]')"
    if [[ "${after_count}" -gt "${before_count}" ]]; then
      echo "${after_count}"
      return 0
    fi
    attempt=$((attempt + 1))
    if ((attempt > 30)); then
      log "ledger_entries did not increase (before=${before_count}, after=${after_count})"
      return 1
    fi
    sleep 2
  done
}

wait_for_route_receipt_increment() {
  local before_count="$1"
  local after_count
  local attempt=0
  while :; do
    after_count="$(query_route_receipt_count | tr -d '[:space:]')"
    if [[ "${after_count}" -gt "${before_count}" ]]; then
      echo "${after_count}"
      return 0
    fi
    attempt=$((attempt + 1))
    if ((attempt > 30)); then
      log "route_receipts did not increase (before=${before_count}, after=${after_count})"
      return 1
    fi
    sleep 2
  done
}

wait_for_route_receipt_diagnostics_increment() {
  local before_count="$1"
  local after_count
  local attempt=0
  while :; do
    after_count="$(query_route_receipt_diagnostics_count | tr -d '[:space:]')"
    if [[ "${after_count}" -gt "${before_count}" ]]; then
      echo "${after_count}"
      return 0
    fi
    attempt=$((attempt + 1))
    if ((attempt > 30)); then
      log "route_receipt_diagnostics did not increase (before=${before_count}, after=${after_count})"
      return 1
    fi
    sleep 2
  done
}

require jq
require curl
require docker

export COMPOSE_FILE="${REPO_ROOT}/infra/docker/compose.yaml"
export COMPOSE_SMOKE_FILE="${REPO_ROOT}/infra/docker/compose.smoke.yaml"
export COMPOSE_PROJECT_NAME="huge-router-smoke-${RUN_ID}"

export POSTGRES_DB="${POSTGRES_DB:-huge_router}"
export POSTGRES_USER="${POSTGRES_USER:-huge_router}"
export POSTGRES_PASSWORD="${POSTGRES_PASSWORD:-huge_router}"
export POSTGRES_PORT="${POSTGRES_PORT:-54330}"
export REDIS_PORT="${REDIS_PORT:-6380}"
export NATS_CLIENT_PORT="${NATS_CLIENT_PORT:-44222}"
export NATS_MONITOR_PORT="${NATS_MONITOR_PORT:-44223}"

export CONTROL_PLANE_API_ADDR="${CONTROL_PLANE_API_ADDR:-127.0.0.1:18081}"
CONTROL_PLANE_HOST="${CONTROL_PLANE_API_ADDR%%:*}"
CONTROL_PLANE_PORT="${CONTROL_PLANE_API_ADDR#*:}"
export CONTROL_PLANE_BASE_URL="http://${CONTROL_PLANE_HOST}:${CONTROL_PLANE_PORT}"
export NATS_URL="nats://127.0.0.1:${NATS_CLIENT_PORT}"
export CONTROL_PLANE_INTERNAL_TOKEN="${CONTROL_PLANE_INTERNAL_TOKEN:-dev-internal-token}"

export CONTROL_PLANE_DATABASE_URL="postgres://${POSTGRES_USER}:${POSTGRES_PASSWORD}@127.0.0.1:${POSTGRES_PORT}/${POSTGRES_DB}"
export LEDGER_WORKER_DATABASE_URL="${CONTROL_PLANE_DATABASE_URL}"
export LEDGER_WORKER_NATS_URL="${NATS_URL}"
export ROUTE_RECEIPT_WORKER_DATABASE_URL="${CONTROL_PLANE_DATABASE_URL}"
export ROUTE_RECEIPT_WORKER_NATS_URL="${NATS_URL}"
export ROUTING_WORKER_DATABASE_URL="${CONTROL_PLANE_DATABASE_URL}"
export ROUTING_WORKER_NATS_URL="${NATS_URL}"
export AUDIT_WORKER_DATABASE_URL="${CONTROL_PLANE_DATABASE_URL}"
export AUDIT_WORKER_NATS_URL="${NATS_URL}"
export NOTIFICATION_WORKER_DATABASE_URL="${CONTROL_PLANE_DATABASE_URL}"
export NOTIFICATION_WORKER_NATS_URL="${NATS_URL}"
export EDGE_PROBE_NATS_URL="${NATS_URL}"
export EDGE_PROBE_TARGET_URL="${EDGE_PROBE_TARGET_URL:-http://control-plane-api:8081/healthz}"
export EDGE_PROBE_PROVIDER_RESOURCE_ID="${EDGE_PROBE_PROVIDER_RESOURCE_ID:-prvrsrc_openai_primary}"
export EDGE_PROBE_PROBE_MODE="${EDGE_PROBE_PROBE_MODE:-cheap_health}"
export EDGE_PROBE_INTERVAL_MS="${EDGE_PROBE_INTERVAL_MS:-30000}"
export EDGE_PROBE_TIMEOUT_MS="${EDGE_PROBE_TIMEOUT_MS:-2000}"
export EDGE_PROBE_DEGRADED_LATENCY_MS="${EDGE_PROBE_DEGRADED_LATENCY_MS:-1500}"

export NATS_URL

COMPOSE_ARGS=(
  -p "${COMPOSE_PROJECT_NAME}"
  -f "${COMPOSE_FILE}"
  -f "${COMPOSE_SMOKE_FILE}"
  --profile runtime
)
for file in \
  "${COMPOSE_FILE}" \
  "${COMPOSE_SMOKE_FILE}"; do
  if [[ ! -f "${file}" ]]; then
    log "missing compose file: ${file}"
    exit 1
  fi
done

DOCKER_STACK_STARTED=false

log "starting smoke dependencies"
docker compose "${COMPOSE_ARGS[@]}" --project-directory "${REPO_ROOT}" up -d postgres redis nats
DOCKER_STACK_STARTED=true

wait_for_http "http://127.0.0.1:${NATS_MONITOR_PORT}/healthz" 120 "nats monitor"
until docker compose "${COMPOSE_ARGS[@]}" exec -T postgres pg_isready -U "${POSTGRES_USER}" -d "${POSTGRES_DB}" >/dev/null 2>&1; do
  sleep 1
done

log "building runtime service images"
docker compose "${COMPOSE_ARGS[@]}" --project-directory "${REPO_ROOT}" build \
  control-plane-api \
  ledger-worker \
  route-receipt-worker \
  routing-worker \
  edge-probe \
  audit-worker \
  notification-worker

log "applying runtime schema and bootstrap"
"${REPO_ROOT}/infra/scripts/migrate.sh" runtime
"${REPO_ROOT}/infra/scripts/bootstrap.sh" runtime

log "starting runtime services"
docker compose "${COMPOSE_ARGS[@]}" --project-directory "${REPO_ROOT}" up -d \
  control-plane-api \
  ledger-worker \
  route-receipt-worker \
  routing-worker \
  edge-probe \
  audit-worker \
  notification-worker

wait_for_http "${CONTROL_PLANE_BASE_URL}/healthz" 180 "control-plane healthz"
wait_for_compose_service_state "ledger-worker" "running" 180
wait_for_compose_service_state "route-receipt-worker" "running" 180
wait_for_compose_service_state "routing-worker" "running" 180
wait_for_compose_service_state "edge-probe" "running" 180
wait_for_compose_service_state "audit-worker" "running" 180
wait_for_compose_service_state "notification-worker" "running" 180
login_platform_admin

log "smoke assertions passed: control-plane + marketplace-support workers healthy"
