# Development Readiness and Bootstrap

[Back to Docs Index](../README.md)

This document freezes the minimum set of decisions required to start implementation without reopening foundational debates on every task.

## 1. Baseline Decisions

As of **April 20, 2026**, the recommended implementation baseline is:

- backend: Rust workspace
- frontend runtime: Node.js `24.x` LTS
- frontend framework: TanStack Start `1.x`, pinned to an exact repository version
- React baseline: `19.2+`
- component foundation: HeroUI `3.x`
- monorepo orchestrator: Turbo
- MCP transport baseline: Streamable HTTP

## 2. Resolved Kickoff Decisions

These decisions should be treated as closed for the initial development window:

1. **Frontend framework risk policy**
   TanStack Start is on the `1.x` line, but the repository still pins exact versions and does not auto-adopt upstream changes without an explicit lockfile and compatibility review.
2. **Northbound OpenAI scope**
   The first gateway slice may ship Chat Completions compatibility first, but IR and route abstractions must be designed around Responses-era concepts so `/v1/responses` can land without a redesign.
3. **MCP transport**
   Implement Streamable HTTP first. Only add deprecated HTTP + SSE compatibility if an actual integration requires it.
4. **Policy engine**
   Keep policy evaluation in-process for the first release. Preserve abstraction boundaries so OPA or another external engine can be added later.
5. **Analytics storage**
   Start with PostgreSQL projections. Defer ClickHouse until query scale or retention needs justify it.
6. **Deployment target**
   Make Docker Compose the first-class developer and self-hosted baseline. Keep Kubernetes as the production-scale target, not a bootstrap blocker.
7. **Plugin roadmap**
   Keep contracts stable early, but treat third-party dynamic plugins as a later phase after internal registries and manifests are proven.

## 3. Version Strategy

### 3.1 JavaScript Stack

- pin TanStack Start and tightly coupled Start packages to exact versions
- pin HeroUI to a specific minor line during bootstrap
- commit `packageManager` in root `package.json`
- use Corepack in local setup and CI
- pin Node.js to `24.15.0`
- pin `pnpm` to `10.33.0`

### 3.2 Runtime Strategy

- use Node.js `24.15.0` LTS in local development, devcontainer, and CI
- use Rust `1.94.1` in local development, devcontainer, and CI
- avoid mixing React 18 and React 19 packages in the workspace
- prefer one React version across `apps/*` and shared UI packages

## 4. Required Files Before Feature Work

Feature implementation should not begin until these files or directories exist:

- `Cargo.toml`
- `package.json`
- `pnpm-workspace.yaml`
- `turbo.json`
- `justfile`
- `docs/runbooks/monorepo-boundaries-and-bootstrap-contracts.md`
- `docs/architecture/open-source-reference-patterns.md`
- `apps/console-web`
- `apps/storybook`
- `packages/design-tokens`
- `packages/ui-kit`
- `packages/ts-api-client`
- `crates/plugin-sdk`
- `crates/plugin-registry`
- `crates/runtime-composition`
- `crates/protocol-ir`
- `crates/provider-traits`
- `services/gateway-api`
- `services/control-plane-api`

## 5. First Week Implementation Order

Recommended implementation order:

1. initialize workspace roots and lock tool versions
2. scaffold `apps/console-web` and `apps/storybook`
3. create `packages/design-tokens` and wire HeroUI 3 theme bootstrap
4. create `packages/ui-kit` shell, feedback, and form primitives
5. add `src/start.ts` with global TanStack Start middleware
6. scaffold `crates/plugin-sdk`, `plugin-registry`, and `runtime-composition`
7. scaffold `crates/protocol-ir` and `provider-traits`
8. scaffold `services/gateway-api` and `services/control-plane-api`
9. wire Turbo tasks, outputs, and CI cache settings
10. add local stack profiles, telemetry bootstrap, and schema generation pipeline

Canonical bootstrap and verification path:

1. `corepack enable`
2. `corepack prepare pnpm@10.33.0 --activate`
3. `pnpm doctor`
4. `pnpm install --frozen-lockfile`
5. `pnpm lint`
6. `pnpm typecheck`
7. `pnpm test`
8. `pnpm build`

## 6. Go/No-Go Checklist

Development is considered ready to start when all of the following are true:

- architecture decisions for Turbo, HeroUI, pluggability, and exact-version policy are documented
- leading open source reference patterns have been translated into repository rules rather than left as informal inspiration
- root workspace files exist
- local frontend can boot with HeroUI theme and TanStack Start middleware
- Rust workspace compiles with placeholder binaries
- Turbo tasks have explicit outputs and package boundaries
- initial JavaScript workspaces declare ownership intent and boundary tags
- Storybook renders shared shell primitives
- schema generation path is defined
- local Postgres, Redis, and NATS stack can start with one command, with observability sidecars available as an optional profile
- `pnpm verify:toolchain` provides one explicit contributor check for Node, `pnpm`, and Rust version drift
- CI plan includes Rust checks, frontend typecheck, tests, and component build verification
- CI can report repository boundary drift before shared packages start proliferating

## 7. What To Defer Deliberately

Do not block implementation on these items:

- dynamic plugin loading
- ClickHouse
- full MCP compatibility matrix across every client transport
- React Server Components in TanStack Start
- broad provider coverage beyond the first adapter
- enterprise-only deployment features

## 8. Immediate Next Coding Tasks

If we start coding now, the recommended first owned slices are:

- frontend-platform: `FND-002`
- backend-platform: `FND-001`
- gateway-architecture: `plugin-sdk`, `plugin-registry`, `runtime-composition`, `protocol-ir`, `provider-traits`
- platform: `FND-003` and `FND-005`

That is enough to enter real development without waiting for later-phase debates.
