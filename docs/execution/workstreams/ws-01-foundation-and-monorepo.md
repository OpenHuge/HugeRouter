# Foundation and Monorepo

[Back to Execution Workstreams](README.md)

Establish repository, tooling, local runtime, schema generation, and CI. This stream unlocks nearly every other stream but can itself be parallelized into Rust workspace, frontend shell, infra stack, and CI automation.

## Task sequence

| Task ID | Phase | Title                                                                    | Depends On       |
| ------- | ----- | ------------------------------------------------------------------------ | ---------------- |
| FND-001 | PI-0  | Initialize Rust workspace and service skeletons                          | None             |
| FND-002 | PI-0  | Initialize pnpm + Turbo workspace and TanStack Start application shell   | None             |
| FND-003 | PI-0  | Provision local development stack and containerized runtime              | None             |
| FND-004 | PI-0  | Establish shared configuration, secrets, and environment loading         | FND-001          |
| FND-005 | PI-0  | Set up CI pipelines, quality gates, and caching strategy                 | FND-001, FND-002 |
| FND-006 | PI-0  | Create schema pipeline for OpenAPI, JSON Schema, and generated TS client | FND-002          |

## Detailed tasks

### FND-001 - Initialize Rust workspace and service skeletons

**Phase:** PI-0  
**Estimated size:** L  
**Recommended owners:** backend-platform

**Depends on:** None

**Primary paths to touch:**

- `Cargo.toml`
- `crates/*`
- `services/*`
- `justfile`

**Expected outputs:**

- Rust workspace root
- service crate templates
- shared lint/test commands

**Acceptance criteria:**

- `cargo check --workspace` passes
- all services compile as placeholder binaries
- workspace dependency policy is documented

**Implementation notes:**

- Keep root tasks minimal and repeatable.
- Prefer generated artifacts and workspace conventions over hand-maintained duplication.
- Optimize for incremental builds and selective CI from day one.

### FND-002 - Initialize pnpm + Turbo workspace and TanStack Start application shell

**Phase:** PI-0  
**Estimated size:** M  
**Recommended owners:** frontend-platform

**Depends on:** None

**Primary paths to touch:**

- `package.json`
- `pnpm-workspace.yaml`
- `turbo.json`
- `apps/console-web`
- `apps/storybook`
- `packages/ui-kit`
- `packages/design-tokens`
- `packages/typescript-config`
- `apps/console-web/src/start.ts`
- `apps/*/turbo.json`
- `packages/*/turbo.json`

**Expected outputs:**

- pnpm workspace root
- Turbo pipeline baseline
- TanStack Start app shell
- HeroUI provider and theme bootstrap
- TanStack Start global middleware bootstrap
- typed route skeleton
- shared TS config
- initial workspace tags and boundary-ready package configurations

**Acceptance criteria:**

- `pnpm install`, `pnpm turbo run dev --filter=console-web`, and `pnpm turbo run typecheck --filter=console-web` work
- React 19.2+ and HeroUI 3 baseline are pinned and compatible
- TanStack Start RC version is pinned exactly
- root layout, auth placeholder, HeroUI theme provider, TanStack Start global middleware, and basic navigation render
- core JavaScript workspaces declare package tags or equivalent ownership metadata
- shared UI primitives can render in Storybook or equivalent isolated component sandbox
- build passes in CI

**Implementation notes:**

- Keep root tasks minimal and repeatable.
- Prefer generated artifacts and workspace conventions over hand-maintained duplication.
- Optimize for incremental builds and selective CI from day one.
- Treat `packages/design-tokens` as the source of truth for HeroUI theme setup and `packages/ui-kit` as the home for reusable shell primitives.
- Keep `Turbo` focused on task orchestration; let package-level scripts stay simple and composable.

### FND-003 - Provision local development stack and containerized runtime

**Phase:** PI-0  
**Estimated size:** M  
**Recommended owners:** platform

**Depends on:** None

**Primary paths to touch:**

- `infra/docker`
- `infra/scripts`
- `.env.example`

**Expected outputs:**

- Docker Compose stack
- local Postgres/Redis/NATS/OTel collectors
- bootstrap scripts

**Acceptance criteria:**

- one command starts local dependencies
- health checks expose readiness
- local stack is documented in runbook

**Implementation notes:**

- Keep root tasks minimal and repeatable.
- Prefer generated artifacts and workspace conventions over hand-maintained duplication.
- Optimize for incremental builds and selective CI from day one.

### FND-004 - Establish shared configuration, secrets, and environment loading

**Phase:** PI-0  
**Estimated size:** M  
**Recommended owners:** backend-platform, security

**Depends on:** FND-001

**Primary paths to touch:**

- `crates/config`
- `crates/sdk-server`
- `docs/runbooks`

**Expected outputs:**

- typed config crate
- env validation
- secret provider interface

**Acceptance criteria:**

- all services load validated config
- missing required variables fail fast
- secret-backed values are separated from non-secret config

**Implementation notes:**

- Keep root tasks minimal and repeatable.
- Prefer generated artifacts and workspace conventions over hand-maintained duplication.
- Optimize for incremental builds and selective CI from day one.

### FND-005 - Set up CI pipelines, quality gates, and caching strategy

**Phase:** PI-0  
**Estimated size:** M  
**Recommended owners:** platform

**Depends on:** FND-001, FND-002

**Primary paths to touch:**

- `.github/workflows`
- `turbo.json`
- `package.json`
- `justfile`

**Expected outputs:**

- CI workflows
- Turbo task graph
- selective change detection
- artifact caching
- remote caching rollout plan
- repository boundary validation rollout

**Acceptance criteria:**

- pull requests run Rust and TS checks
- jobs are split by workspace scope and use Turbo filters where appropriate
- cacheable Turbo tasks declare `outputs`
- advisory repository boundary checks run in CI for tagged workspaces
- cache hit rate is measurable

**Implementation notes:**

- Keep root tasks minimal and repeatable.
- Prefer generated artifacts and workspace conventions over hand-maintained duplication.
- Optimize for incremental builds and selective CI from day one.
- Cache JavaScript work at the Turbo layer and Rust work at the Cargo layer instead of trying to force a single cache abstraction.
- Start repository-boundary checks in advisory mode so teams can fix drift without blocking the initial bootstrap.

### FND-006 - Create schema pipeline for OpenAPI, JSON Schema, and generated TS client

**Phase:** PI-0  
**Estimated size:** M  
**Recommended owners:** platform, frontend-platform

**Depends on:** FND-002

**Primary paths to touch:**

- `schemas/openapi`
- `schemas/jsonschema`
- `packages/ts-api-client`
- `packages/ts-shared-schema`

**Expected outputs:**

- schema source-of-truth workflow
- TS client generation
- schema linting

**Acceptance criteria:**

- OpenAPI documents validate
- TS client can be regenerated from CI
- console consumes generated types

**Implementation notes:**

- Keep root tasks minimal and repeatable.
- Prefer generated artifacts and workspace conventions over hand-maintained duplication.
- Optimize for incremental builds and selective CI from day one.
