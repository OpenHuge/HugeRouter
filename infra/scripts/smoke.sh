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
    if [[ "${code}" -ne 0 ]]; then
      docker compose "${COMPOSE_ARGS[@]}" ps >"${LOG_ROOT}/compose-ps.txt" 2>&1 || true
      docker compose "${COMPOSE_ARGS[@]}" logs --no-color >"${LOG_ROOT}/compose.log" 2>&1 || true
    fi
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
        supports_json_mode: true,
        supports_realtime: false,
        supports_response_model_metadata: true
      },
      supported_protocol_families: ["openai_chat", "openai_responses"],
      is_transit_gateway: false,
      version: 1,
      created_at: "2026-04-22T00:00:00Z",
      updated_at: "2026-04-22T00:00:00Z"
    }')"

  curl -fsS -X POST "${CONTROL_PLANE_BASE_URL}/v1/provider-resources" \
    -b "${CONTROL_PLANE_COOKIE_JAR}" \
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
    -b "${CONTROL_PLANE_COOKIE_JAR}" \
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
    -b "${CONTROL_PLANE_COOKIE_JAR}" \
    -H "content-type: application/json" \
    --data "${payload}" >/dev/null
  curl -fsS -X POST "${CONTROL_PLANE_BASE_URL}/v1/config-snapshots/${config_snapshot_id}/activate" \
    -b "${CONTROL_PLANE_COOKIE_JAR}" >/dev/null
}

create_gateway_api_key() {
  local provider_resource_id="$1"
  local api_key="$2"
  local display_name="$3"
  local payload
  payload="$(jq -n \
    --arg provider_resource_id "${provider_resource_id}" \
    --arg display_name "${display_name}" \
    --arg api_key "${api_key}" \
    '{provider_resource_id:$provider_resource_id,display_name:$display_name,api_key:$api_key}')"

  curl -fsS -X POST "${CONTROL_PLANE_BASE_URL}/v1/api-keys" \
    -b "${CONTROL_PLANE_COOKIE_JAR}" \
    -H "content-type: application/json" \
    --data "${payload}" >/dev/null
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

run_gateway_smoke_request() {
  local output_file="$1"
  local max_attempts=5
  local attempt=1
  local http_status

  while :; do
    http_status="$(curl -sS -o "${output_file}" -w "%{http_code}" \
      -X POST "${GATEWAY_BASE_URL}/v1/responses" \
      -H "authorization: Bearer ${SMOKE_API_KEY}" \
      -H "content-type: application/json" \
      --data "{\"model\":\"${SMOKE_OPENAI_MODEL_ALIAS}\",\"input\":[{\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"smoke\"}]}],\"stream\":false}")"

    if [[ "${http_status}" == "${SMOKE_EXPECT_GATEWAY_STATUS}" ]]; then
      echo "${http_status}"
      return 0
    fi

    if [[ "${SMOKE_EXPECT_GATEWAY_STATUS}" == "200" && "${http_status}" =~ ^50[234]$ && "${attempt}" -lt "${max_attempts}" ]]; then
      echo "[smoke] gateway request retry ${attempt}/${max_attempts} after transient http=${http_status}" >&2
      attempt=$((attempt + 1))
      sleep 2
      continue
    fi

    echo "${http_status}"
    return 0
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
export GATEWAY_API_ADDR="${GATEWAY_API_ADDR:-127.0.0.1:18080}"
CONTROL_PLANE_HOST="${CONTROL_PLANE_API_ADDR%%:*}"
CONTROL_PLANE_PORT="${CONTROL_PLANE_API_ADDR#*:}"
GATEWAY_HOST="${GATEWAY_API_ADDR%%:*}"
GATEWAY_PORT="${GATEWAY_API_ADDR#*:}"
export CONTROL_PLANE_PORT
export GATEWAY_PORT
export CONTROL_PLANE_BASE_URL="http://${CONTROL_PLANE_HOST}:${CONTROL_PLANE_PORT}"
export GATEWAY_BASE_URL="http://${GATEWAY_HOST}:${GATEWAY_PORT}"
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
export EDGE_PROBE_TARGET_URL="${EDGE_PROBE_TARGET_URL:-http://gateway-api:8080/healthz}"
export EDGE_PROBE_PROVIDER_RESOURCE_ID="${EDGE_PROBE_PROVIDER_RESOURCE_ID:-prvrsrc_openai_primary}"
export EDGE_PROBE_PROBE_MODE="${EDGE_PROBE_PROBE_MODE:-cheap_health}"
export EDGE_PROBE_INTERVAL_MS="${EDGE_PROBE_INTERVAL_MS:-30000}"
export EDGE_PROBE_TIMEOUT_MS="${EDGE_PROBE_TIMEOUT_MS:-2000}"
export EDGE_PROBE_DEGRADED_LATENCY_MS="${EDGE_PROBE_DEGRADED_LATENCY_MS:-1500}"
export GATEWAY_NATS_URL="${NATS_URL}"

SMOKE_OPENAI_API_KEY="${SMOKE_OPENAI_API_KEY:-${OPENAI_API_KEY:-}}"
if [[ -z "${SMOKE_OPENAI_API_KEY}" ]]; then
  log "OPENAI_API_KEY / SMOKE_OPENAI_API_KEY is required for full usage-event smoke path"
  exit 1
fi
export SMOKE_OPENAI_API_KEY
export SMOKE_OPENAI_BASE_URL="${SMOKE_OPENAI_BASE_URL:-https://api.openai.com/v1}"
export SMOKE_OPENAI_MODEL_ALIAS="${SMOKE_OPENAI_MODEL_ALIAS:-reasoning-fast}"
export SMOKE_OPENAI_WIRE_API="${SMOKE_OPENAI_WIRE_API:-responses}"
export SMOKE_EXPECT_GATEWAY_STATUS="${SMOKE_EXPECT_GATEWAY_STATUS:-200}"
export GATEWAY_OPENAI_API_KEY="${SMOKE_OPENAI_API_KEY}"
export GATEWAY_OPENAI_MODEL="${GATEWAY_OPENAI_MODEL:-${SMOKE_OPENAI_MODEL_ALIAS}}"
export GATEWAY_OPENAI_WIRE_API="${SMOKE_OPENAI_WIRE_API}"
export OPENAI_API_KEY="${SMOKE_OPENAI_API_KEY}"

# Keep these in-place for future adapter expansion and optional smoke variants.
export SMOKE_ANTHROPIC_API_KEY="${SMOKE_ANTHROPIC_API_KEY:-${ANTHROPIC_API_KEY:-}}"
export SMOKE_GEMINI_API_KEY="${SMOKE_GEMINI_API_KEY:-${GEMINI_API_KEY:-}}"
export GATEWAY_ANTHROPIC_API_KEY="${SMOKE_ANTHROPIC_API_KEY}"
export GATEWAY_GEMINI_API_KEY="${SMOKE_GEMINI_API_KEY}"

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
docker compose "${COMPOSE_ARGS[@]}" up -d postgres redis nats
DOCKER_STACK_STARTED=true

wait_for_http "http://127.0.0.1:${NATS_MONITOR_PORT}/healthz" 120 "nats monitor"
until docker compose "${COMPOSE_ARGS[@]}" exec -T postgres pg_isready -U "${POSTGRES_USER}" -d "${POSTGRES_DB}" >/dev/null 2>&1; do
  sleep 1
done

log "building runtime service images"
docker compose "${COMPOSE_ARGS[@]}" build \
  control-plane-api \
  gateway-api \
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
docker compose "${COMPOSE_ARGS[@]}" up -d \
  control-plane-api \
  gateway-api \
  ledger-worker \
  route-receipt-worker \
  routing-worker \
  edge-probe \
  audit-worker \
  notification-worker

wait_for_http "${CONTROL_PLANE_BASE_URL}/healthz" 180 "control-plane healthz"
wait_for_http "${GATEWAY_BASE_URL}/healthz" 180 "gateway healthz"
wait_for_compose_service_state "ledger-worker" "running" 180
wait_for_compose_service_state "route-receipt-worker" "running" 180
wait_for_compose_service_state "routing-worker" "running" 180
wait_for_compose_service_state "edge-probe" "running" 180
wait_for_compose_service_state "audit-worker" "running" 180
wait_for_compose_service_state "notification-worker" "running" 180
login_platform_admin

log "running smoke openai setup"
provider_resource_id="prvrsrc_smoke_openai"
route_policy_id="routepol_smoke_openai"
config_snapshot_id="cfgsnap_smoke_openai"
create_provider_resource "openai" "${provider_resource_id}" "smoke-openai-${RUN_ID}" "${SMOKE_OPENAI_BASE_URL}"
create_route_policy "${route_policy_id}" "openai_chat" "${SMOKE_OPENAI_MODEL_ALIAS}" "smoke-openai-policy"
create_and_activate_snapshot "${config_snapshot_id}" "${route_policy_id}" "${provider_resource_id}" 2

SMOKE_API_KEY="sk_smoke_${RUN_ID}_${RANDOM}_${RANDOM}"
create_key_payload="$(jq -n \
  --arg provider_resource_id "${provider_resource_id}" \
  --arg display_name "smoke-${RUN_ID}" \
  --arg api_key "${SMOKE_API_KEY}" \
  '{provider_resource_id:$provider_resource_id,display_name:$display_name,api_key:$api_key}')"

create_status="$(curl -sS -o "${LOG_ROOT}/create_api_key.json" -w "%{http_code}" \
  -X POST "${CONTROL_PLANE_BASE_URL}/v1/api-keys" \
  -b "${CONTROL_PLANE_COOKIE_JAR}" \
  -H "content-type: application/json" \
  --data "${create_key_payload}")"
if [[ "${create_status}" != "200" ]]; then
  log "create api key failed, http=${create_status}"
  log "response: $(cat "${LOG_ROOT}/create_api_key.json")"
  exit 1
fi

route_receipts_before="$(query_route_receipt_count | tr -d '[:space:]')"
route_receipt_diagnostics_before="$(query_route_receipt_diagnostics_count | tr -d '[:space:]')"
ledger_before="$(query_ledger_count | tr -d '[:space:]')"
GATEWAY_STATUS="$(run_gateway_smoke_request "${LOG_ROOT}/gateway_request.json")"
if [[ "${GATEWAY_STATUS}" != "${SMOKE_EXPECT_GATEWAY_STATUS}" ]]; then
  log "gateway request failed, http=${GATEWAY_STATUS}"
  log "response: $(cat "${LOG_ROOT}/gateway_request.json")"
  exit 1
fi

route_receipts_after="$(wait_for_route_receipt_increment "${route_receipts_before}")"
route_receipt_diagnostics_after="$(
  wait_for_route_receipt_diagnostics_increment "${route_receipt_diagnostics_before}"
)"
if [[ "${SMOKE_EXPECT_GATEWAY_STATUS}" == "200" ]]; then
  if ! jq -e '.usage.total_tokens and .output_text' "${LOG_ROOT}/gateway_request.json" >/dev/null 2>&1; then
    log "gateway response missing usage block: $(cat "${LOG_ROOT}/gateway_request.json")"
    exit 1
  fi
  ledger_after="$(wait_for_ledger_increment "${ledger_before}")"
else
  if ! jq -e '.error.code' "${LOG_ROOT}/gateway_request.json" >/dev/null 2>&1; then
    log "gateway error response missing normalized error block: $(cat "${LOG_ROOT}/gateway_request.json")"
    exit 1
  fi
  ledger_after="${ledger_before}"
fi

run_optional_protocol_smoke() {
  local provider_id="$1"
  local protocol_family="$2"
  local model_alias="$3"
  local provider_endpoint="$4"
  local gateway_path="$5"
  local request_payload="$6"
  local response_jq="$7"
  local provider_resource_id="$8"
  local route_policy_id="$9"
  local config_snapshot_id="${10}"
  local gateway_api_key="${11}"

  if [[ -z "${gateway_api_key}" ]]; then
    log "skipping ${provider_id} smoke; API key not provided"
    return 0
  fi

  log "running ${provider_id} smoke path"
  create_provider_resource "${provider_id}" "${provider_resource_id}" "smoke-${provider_id}-${RUN_ID}" "${provider_endpoint}"
  create_route_policy "${route_policy_id}" "${protocol_family}" "${model_alias}" "smoke-${provider_id}-policy"
  create_and_activate_snapshot "${config_snapshot_id}" "${route_policy_id}" "${provider_resource_id}" 2

  local smoke_api_key="sk_smoke_${provider_id}_${RUN_ID}_${RANDOM}"
  create_gateway_api_key "${provider_resource_id}" "${smoke_api_key}" "smoke-${provider_id}-${RUN_ID}"

  local before_count
  before_count="$(query_ledger_count | tr -d '[:space:]')"
  local before_route_receipts
  before_route_receipts="$(query_route_receipt_count | tr -d '[:space:]')"
  local before_route_receipt_diagnostics
  before_route_receipt_diagnostics="$(query_route_receipt_diagnostics_count | tr -d '[:space:]')"
  local response_file="${LOG_ROOT}/${provider_id}_gateway_request.json"
  local http_status
  http_status="$(curl -sS -o "${response_file}" -w "%{http_code}" \
    -X POST "${GATEWAY_BASE_URL}${gateway_path}" \
    -H "authorization: Bearer ${smoke_api_key}" \
    -H "content-type: application/json" \
    --data "${request_payload}")"
  if [[ "${http_status}" != "200" ]]; then
    log "${provider_id} gateway request failed, http=${http_status}"
    log "response: $(cat "${response_file}")"
    exit 1
  fi

  if ! jq -e "${response_jq}" "${response_file}" >/dev/null 2>&1; then
    log "${provider_id} response did not match expected shape: $(cat "${response_file}")"
    exit 1
  fi

  local after_count
  after_count="$(wait_for_ledger_increment "${before_count}")"
  local after_route_receipts
  after_route_receipts="$(wait_for_route_receipt_increment "${before_route_receipts}")"
  local after_route_receipt_diagnostics
  after_route_receipt_diagnostics="$(
    wait_for_route_receipt_diagnostics_increment "${before_route_receipt_diagnostics}"
  )"
  log "${provider_id} smoke assertions passed (ledger ${before_count} -> ${after_count}, route_receipts ${before_route_receipts} -> ${after_route_receipts}, route_receipt_diagnostics ${before_route_receipt_diagnostics} -> ${after_route_receipt_diagnostics})"
}

run_optional_protocol_smoke \
  "anthropic" \
  "anthropic_messages" \
  "claude-3-opus" \
  "https://api.anthropic.com/v1" \
  "/v1/messages" \
  '{"model":"claude-3-opus","messages":[{"role":"user","content":[{"type":"text","text":"smoke anthropic"}]}],"max_tokens":128,"stream":false}' \
  '.content[0].text' \
  "prvrsrc_smoke_anthropic" \
  "routepol_smoke_anthropic" \
  "cfgsnap_smoke_anthropic" \
  "${SMOKE_ANTHROPIC_API_KEY}"

run_optional_protocol_smoke \
  "gemini" \
  "gemini_generate_content" \
  "gemini-1.5-pro" \
  "https://generativelanguage.googleapis.com" \
  "/v1beta/models/gemini-1.5-pro:generateContent" \
  '{"model":"gemini-1.5-pro","contents":[{"role":"user","parts":[{"text":"smoke gemini"}]}],"tools":[],"stream":false}' \
  '.candidates[0].content.parts[0].text' \
  "prvrsrc_smoke_gemini" \
  "routepol_smoke_gemini" \
  "cfgsnap_smoke_gemini" \
  "${SMOKE_GEMINI_API_KEY}"

log "smoke assertions passed: control-plane + gateway + ledger worker + route receipt worker healthy"
log "provider resource used: ${provider_resource_id}"
log "ledger count: ${ledger_before} -> ${ledger_after}"
log "route receipt count: ${route_receipts_before} -> ${route_receipts_after}"
log "route receipt diagnostics count: ${route_receipt_diagnostics_before} -> ${route_receipt_diagnostics_after}"
