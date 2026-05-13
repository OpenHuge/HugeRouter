#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/stack-common.sh"

mode="$(resolve_stack_mode "${1:-runtime}")"
if [[ "${mode}" == "observability" ]]; then
  echo "bootstrap.sh does not support observability-only mode." >&2
  exit 1
fi

"${SCRIPT_DIR}/migrate.sh" "${mode}"
stack_exec_sql_file "${mode}" "${STACK_SQL_BOOTSTRAP_FILE}"

printf 'Applied runtime bootstrap data using stack mode %s.\n' "${mode}"
