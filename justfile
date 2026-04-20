set shell := ["powershell.exe", "-NoLogo", "-Command"]

bootstrap:
  corepack enable
  corepack prepare pnpm@10.33.0 --activate
  pnpm install
  node ./scripts/run-cargo.mjs check --workspace

dev-frontend:
  pnpm turbo run dev --filter=console-web

dev-backend:
  node ./scripts/run-cargo.mjs run -p gateway-api

dev-ui:
  pnpm turbo run storybook --filter=storybook

dev-all:
  pnpm turbo run dev --parallel --filter=console-web --filter=storybook

test:
  node ./scripts/run-cargo.mjs test --workspace
  pnpm turbo run test

lint:
  node ./scripts/run-cargo.mjs clippy --workspace --all-targets
  pnpm turbo run lint

fmt:
  node ./scripts/run-cargo.mjs fmt --all
  pnpm exec prettier --write .

generate:
  pnpm turbo run generate

stack-up:
  ./infra/scripts/stack-up.ps1

stack-down:
  ./infra/scripts/stack-down.ps1

stack-logs:
  ./infra/scripts/stack-logs.ps1
