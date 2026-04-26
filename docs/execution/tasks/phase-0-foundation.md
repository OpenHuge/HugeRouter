# PI-0 - Foundation

[Back to Task Catalog Index](README.md)

Foundation tasks that unlock all other streams.

## Recommended execution order

- **CTL-001** - Create control plane API service skeleton with typed CRUD conventions _(depends on: FND-001, FND-004)_
- **CTL-004** - Build TanStack Start + HeroUI application shell and authenticated app layout _(depends on: FND-002, FND-006)_
- **DB-001** - Design relational schema, migrations, and repository primitives _(depends on: FND-001, FND-003)_
- **DB-002** - Implement event bus abstractions and NATS-backed publishers/consumers _(depends on: FND-001, FND-003)_
- **FND-001** - Initialize Rust workspace and service skeletons _(depends on: None)_
- **FND-002** - Initialize pnpm + Turbo workspace and TanStack Start application shell _(depends on: None)_
- **FND-003** - Provision local development stack and containerized runtime _(depends on: None)_
- **FND-004** - Establish shared configuration, secrets, and environment loading _(depends on: FND-001)_
- **FND-005** - Set up CI pipelines, quality gates, and caching strategy _(depends on: FND-001, FND-002)_
- **FND-006** - Create schema pipeline for OpenAPI, JSON Schema, and generated TS client _(depends on: FND-002)_
- **OBS-001** - Implement telemetry crate and platform-wide OpenTelemetry conventions _(depends on: FND-001, FND-003)_
- **QAR-004** - Create agent-facing implementation guides and definition-of-done checklists _(depends on: None)_
- **SEC-001** - Define RBAC model, scopes, and authorization middleware _(depends on: FND-001, CTL-001)_

## Detailed tasks

### CTL-001 - Create control plane API service skeleton with typed CRUD conventions

**Stream:** Control Plane and Console  
**Owners:** control-plane  
**Depends on:** FND-001, FND-004

**Paths:**

- `services/control-plane-api`
- `crates/sdk-server`
- `schemas/openapi`

**Outputs:**

- service bootstrap
- error envelope
- pagination/filtering conventions

**Acceptance criteria:**

- service exposes versioned base path
- OpenAPI spec is generated from implementation
- integration test can start service against local stack

### CTL-004 - Build TanStack Start + HeroUI application shell and authenticated app layout

**Stream:** Control Plane and Console  
**Owners:** frontend-console  
**Depends on:** FND-002, FND-006

**Paths:**

- `apps/console-web`
- `packages/ui-kit`
- `packages/design-tokens`

**Outputs:**

- route tree
- shell layout
- HeroUI-backed theme and navigation system
- shared provider stack and shell primitives

**Acceptance criteria:**

- authenticated and unauthenticated layouts render
- internal HeroUI-compatible shell or equivalent shell wrapper is in place
- route-level data loaders compile
- component story coverage exists for shell primitives

### DB-001 - Design relational schema, migrations, and repository primitives

**Stream:** Data Storage and Events  
**Owners:** data-platform  
**Depends on:** FND-001, FND-003

**Paths:**

- `crates/storage`
- `infra/docker`
- `docs/architecture/data-storage-and-events.md`

**Outputs:**

- migration framework
- core tables
- repository abstractions

**Acceptance criteria:**

- fresh database bootstraps successfully
- rollback policy is documented
- repositories are integration-tested

### DB-002 - Implement event bus abstractions and NATS-backed publishers/consumers

**Stream:** Data Storage and Events  
**Owners:** data-platform  
**Depends on:** FND-001, FND-003

**Paths:**

- `crates/queue`
- `services/*`
- `crates/sdk-server`

**Outputs:**

- event publishing API
- consumer worker harness
- subject naming conventions

**Acceptance criteria:**

- workers can consume and ack events
- message schemas are versioned
- dead-letter strategy is defined

### FND-001 - Initialize Rust workspace and service skeletons

**Stream:** Foundation and Monorepo  
**Owners:** backend-platform  
**Depends on:** None

**Paths:**

- `Cargo.toml`
- `crates/*`
- `services/*`
- `justfile`

**Outputs:**

- Rust workspace root
- service crate templates
- shared lint/test commands

**Acceptance criteria:**

- `cargo check --workspace` passes
- all services compile as placeholder binaries
- workspace dependency policy is documented

### FND-002 - Initialize pnpm + Turbo workspace and TanStack Start application shell

**Stream:** Foundation and Monorepo  
**Owners:** frontend-platform  
**Depends on:** None

**Paths:**

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

**Outputs:**

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

### FND-003 - Provision local development stack and containerized runtime

**Stream:** Foundation and Monorepo  
**Owners:** platform  
**Depends on:** None

**Paths:**

- `infra/docker`
- `infra/scripts`
- `.env.example`

**Outputs:**

- Docker Compose stack
- local Postgres/Redis/NATS/OTel collectors
- bootstrap scripts

**Acceptance criteria:**

- one command starts local dependencies
- health checks expose readiness
- local stack is documented in runbook

### FND-004 - Establish shared configuration, secrets, and environment loading

**Stream:** Foundation and Monorepo  
**Owners:** backend-platform, security  
**Depends on:** FND-001

**Paths:**

- `crates/config`
- `crates/sdk-server`
- `docs/runbooks`

**Outputs:**

- typed config crate
- env validation
- secret provider interface

**Acceptance criteria:**

- all services load validated config
- missing required variables fail fast
- secret-backed values are separated from non-secret config

### FND-005 - Set up CI pipelines, quality gates, and caching strategy

**Stream:** Foundation and Monorepo  
**Owners:** platform  
**Depends on:** FND-001, FND-002

**Paths:**

- `.github/workflows`
- `turbo.json`
- `package.json`
- `justfile`

**Outputs:**

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

### FND-006 - Create schema pipeline for OpenAPI, JSON Schema, and generated TS client

**Stream:** Foundation and Monorepo  
**Owners:** platform, frontend-platform  
**Depends on:** FND-002

**Paths:**

- `schemas/openapi`
- `schemas/jsonschema`
- `packages/ts-api-client`
- `packages/ts-shared-schema`

**Outputs:**

- schema source-of-truth workflow
- TS client generation
- schema linting

**Acceptance criteria:**

- OpenAPI documents validate
- TS client can be regenerated from CI
- console consumes generated types

### OBS-001 - Implement telemetry crate and platform-wide OpenTelemetry conventions

**Stream:** Observability, SRE, and Runtime  
**Owners:** sre  
**Depends on:** FND-001, FND-003

**Paths:**

- `crates/telemetry`
- `infra/monitoring`
- `services/*`

**Outputs:**

- trace helpers
- metrics registry
- structured log conventions

**Acceptance criteria:**

- all services emit traces, metrics, and logs with shared resource tags
- trace IDs propagate across HTTP and events
- collector config is documented

### QAR-004 - Create agent-facing implementation guides and definition-of-done checklists

**Stream:** QA, Release, and Documentation  
**Owners:** technical-writing  
**Depends on:** None

**Paths:**

- `docs/execution`
- `README.md`

**Outputs:**

- execution docs
- task templates
- checklists

**Acceptance criteria:**

- agents can discover work without reading the whole spec
- task handoff template exists
- checklists align with CI and review requirements

### SEC-001 - Define RBAC model, scopes, and authorization middleware

**Stream:** Security, Identity, and Compliance  
**Owners:** security  
**Depends on:** FND-001, CTL-001

**Paths:**

- `crates/authn-authz`
- `services/control-plane-api`
- `services/gateway-api`

**Outputs:**

- role model
- scope evaluation
- authorization middleware

**Acceptance criteria:**

- tenant and platform scopes are distinct
- forbidden responses are normalized
- authorization decisions are traceable
