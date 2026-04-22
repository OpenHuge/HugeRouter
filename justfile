set shell := ["bash", "-euo", "pipefail", "-c"]
set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

stack_up_command := if os_family() == "windows" { ".\\infra\\scripts\\stack-up.ps1" } else { "./infra/scripts/stack-up.sh" }
stack_down_command := if os_family() == "windows" { ".\\infra\\scripts\\stack-down.ps1" } else { "./infra/scripts/stack-down.sh" }
stack_logs_command := if os_family() == "windows" { ".\\infra\\scripts\\stack-logs.ps1" } else { "./infra/scripts/stack-logs.sh" }

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
  node ./scripts/run-cargo.mjs run -p gateway-api

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

stack-up:
  {{stack_up_command}}

stack-down:
  {{stack_down_command}}

stack-logs:
  {{stack_logs_command}}
