set shell := ["bash", "-euo", "pipefail", "-c"]
set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

stack_up_command := if os_family() == "windows" { ".\\infra\\scripts\\stack-up.ps1" } else { "./infra/scripts/stack-up.sh" }
stack_down_command := if os_family() == "windows" { ".\\infra\\scripts\\stack-down.ps1" } else { "./infra/scripts/stack-down.sh" }
stack_logs_command := if os_family() == "windows" { ".\\infra\\scripts\\stack-logs.ps1" } else { "./infra/scripts/stack-logs.sh" }
stack_wait_command := if os_family() == "windows" { ".\\infra\\scripts\\wait-for-stack.ps1" } else { "./infra/scripts/wait-for-stack.sh" }
stack_status_command := if os_family() == "windows" { ".\\infra\\scripts\\stack-status.ps1" } else { "./infra/scripts/stack-status.sh" }
stack_migrate_command := if os_family() == "windows" { ".\\infra\\scripts\\migrate.ps1" } else { "./infra/scripts/migrate.sh" }
stack_bootstrap_command := if os_family() == "windows" { ".\\infra\\scripts\\bootstrap.ps1" } else { "./infra/scripts/bootstrap.sh" }
smoke_command := if os_family() == "windows" { "bash ./infra/scripts/smoke.sh" } else { "./infra/scripts/smoke.sh" }
default_stack_mode := env_var_or_default("HUGE_ROUTER_STACK_MODE", "core")

bootstrap:
  corepack enable
  corepack prepare pnpm@10.33.0 --activate
  pnpm verify:toolchain
  pnpm install --frozen-lockfile
  pnpm rust:check

doctor:
  pnpm verify:toolchain

dev-frontend:
  pnpm turbo run dev --filter=console-web

dev-backend:
  node ./scripts/run-cargo.mjs run -p control-plane-api

dev-ui:
  pnpm turbo run storybook --filter=storybook

dev-all:
  pnpm turbo run dev --parallel --filter=console-web --filter=storybook

test:
  pnpm test

lint:
  pnpm lint

typecheck:
  pnpm typecheck

build:
  pnpm build

fmt:
  node ./scripts/run-cargo.mjs fmt --all
  pnpm exec prettier --write .

generate:
  pnpm generate

stack-up mode=default_stack_mode:
  {{stack_up_command}} {{mode}}

stack-up-full:
  {{stack_up_command}} full

stack-up-runtime:
  {{stack_up_command}} runtime

stack-up-observability:
  {{stack_up_command}} observability

stack-down:
  {{stack_down_command}}

stack-logs mode=default_stack_mode:
  {{stack_logs_command}} {{mode}}

stack-wait mode=default_stack_mode:
  {{stack_wait_command}} {{mode}}

migrate mode="runtime":
  {{stack_migrate_command}} {{mode}}

stack-bootstrap mode="runtime":
  {{stack_bootstrap_command}} {{mode}}

status mode=default_stack_mode:
  {{stack_status_command}} {{mode}}

smoke:
  {{smoke_command}}
