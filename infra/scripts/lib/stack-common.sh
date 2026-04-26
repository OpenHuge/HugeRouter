#!/usr/bin/env bash
set -euo pipefail

readonly STACK_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly STACK_REPO_ROOT="$(cd "${STACK_SCRIPT_DIR}/../../.." && pwd)"
readonly STACK_COMPOSE_FILE="${STACK_REPO_ROOT}/infra/docker/compose.yaml"
readonly STACK_SQL_SCHEMA_FILE="${STACK_REPO_ROOT}/infra/sql/runtime-schema.sql"
readonly STACK_SQL_BOOTSTRAP_FILE="${STACK_REPO_ROOT}/infra/sql/runtime-bootstrap.sql"

resolve_stack_mode() {
  local mode="${1:-${HUGE_ROUTER_STACK_MODE:-core}}"

  case "${mode}" in
    core | runtime | full | observability)
      printf '%s\n' "${mode}"
      ;;
    *)
      echo "Unsupported stack mode: ${mode}. Expected one of: core, runtime, full, observability." >&2
      return 1
      ;;
  esac
}

stack_services_for_mode() {
  case "$1" in
    core)
      printf '%s\n' postgres redis nats
      ;;
    runtime)
      printf '%s\n' \
        postgres redis nats \
        control-plane-api gateway-api ledger-worker route-receipt-worker \
        routing-worker edge-probe audit-worker notification-worker
      ;;
    observability)
      printf '%s\n' otel-collector alertmanager prometheus grafana
      ;;
    full)
      printf '%s\n' \
        postgres redis nats \
        control-plane-api gateway-api ledger-worker route-receipt-worker \
        routing-worker edge-probe audit-worker notification-worker \
        otel-collector alertmanager prometheus grafana
      ;;
    *)
      echo "Unsupported stack mode: $1" >&2
      return 1
      ;;
  esac
}

stack_profiles_for_mode() {
  case "$1" in
    core)
      ;;
    runtime)
      printf '%s\n' runtime
      ;;
    observability)
      printf '%s\n' observability
      ;;
    full)
      printf '%s\n' runtime observability
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

  local -a compose_args=()
  [[ -f "${STACK_REPO_ROOT}/.env" ]] && compose_args+=(--env-file "${STACK_REPO_ROOT}/.env")
  [[ -f "${STACK_REPO_ROOT}/.env.local" ]] && compose_args+=(--env-file "${STACK_REPO_ROOT}/.env.local")
  compose_args+=(-f "${STACK_COMPOSE_FILE}")

  while IFS= read -r profile; do
    [[ -n "${profile}" ]] && compose_args+=(--profile "${profile}")
  done < <(stack_profiles_for_mode "${mode}")

  docker compose "${compose_args[@]}" "$@"
}

stack_postgres_user() {
  printf '%s\n' "${POSTGRES_USER:-huge_router}"
}

stack_postgres_db() {
  printf '%s\n' "${POSTGRES_DB:-huge_router}"
}

stack_postgres_password() {
  printf '%s\n' "${POSTGRES_PASSWORD:-huge_router}"
}

wait_for_postgres() {
  local mode="${1:-core}"
  local timeout_seconds="${2:-${HUGE_ROUTER_STACK_TIMEOUT_SECONDS:-180}}"
  local deadline=$((SECONDS + timeout_seconds))
  local postgres_user
  local postgres_db

  postgres_user="$(stack_postgres_user)"
  postgres_db="$(stack_postgres_db)"

  while ((SECONDS < deadline)); do
    if stack_compose "${mode}" exec -T postgres \
      pg_isready -U "${postgres_user}" -d "${postgres_db}" >/dev/null 2>&1; then
      return 0
    fi
    sleep 2
  done

  echo "Timed out waiting for postgres after ${timeout_seconds}s." >&2
  return 1
}

stack_exec_sql_file() {
  local mode="$1"
  local sql_file="$2"

  if [[ ! -f "${sql_file}" ]]; then
    echo "Missing SQL file: ${sql_file}" >&2
    return 1
  fi

  local postgres_user
  local postgres_db
  local postgres_password

  postgres_user="$(stack_postgres_user)"
  postgres_db="$(stack_postgres_db)"
  postgres_password="$(stack_postgres_password)"

  stack_compose "${mode}" exec -T \
    -e "PGPASSWORD=${postgres_password}" \
    postgres \
    psql \
      -h 127.0.0.1 \
      -U "${postgres_user}" \
      -d "${postgres_db}" \
      -v ON_ERROR_STOP=1 \
      -f - < "${sql_file}"
}
