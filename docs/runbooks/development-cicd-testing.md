# 29. Local Development Experience

[Back to Docs Index](../README.md)

### 29.1 Developer Expectations

A new engineer should be able to:

- install the repo
- start local dependencies
- run gateway and control plane
- run the TanStack Start frontend
- work on shared Mantine components in isolation
- call example APIs
- inspect traces and logs
- run tests and linters

### 29.2 Local Stack

Recommended local stack via Docker Compose:

- PostgreSQL
- Redis
- NATS or Kafka
- optional OpenTelemetry collector
- optional Prometheus
- optional Grafana
- optional ClickHouse

### 29.3 Workspace Tooling Baseline

The repo should operate with:

- `cargo` for Rust workspace commands
- `pnpm` for JavaScript package management
- `turbo` for frontend and shared-package task orchestration
- `just` as the top-level command surface for common workflows

Recommended JavaScript runtime baseline:

- Node.js `24.15.0` in local development, devcontainer, and CI
- Corepack-enabled `pnpm 10.33.0`
- React `19.2+` because Mantine 9 requires it
- TanStack Start pinned to an exact RC version until 1.0 stable is available

Recommended Rust runtime baseline:

- Rust `1.94.1` via `rust-toolchain.toml`
- `rustfmt` and `clippy` installed in local development, devcontainer, and CI

### 29.4 Day-0 Bootstrap Flow

Once the repository scaffolding exists, the expected bootstrap sequence is:

```text
corepack enable
corepack prepare pnpm@10.33.0 --activate
pnpm doctor
pnpm install --frozen-lockfile
pnpm lint
pnpm typecheck
pnpm test
pnpm build
just stack-up
just stack-wait
just stack-up-full
just dev-frontend
just dev-backend
```

The frontend bootstrap should verify:

- `apps/console-web` starts locally
- the root Mantine provider renders with project theme tokens
- `src/start.ts` successfully registers global Start middleware
- Storybook can render `packages/ui-kit` primitives and shell components
- generated TypeScript clients and schemas can be consumed without manual patching

### 29.5 Suggested Task Commands

Suggested `just` tasks:

```text
just bootstrap
just doctor
just build
just dev-backend
just dev-frontend
just dev-ui
just dev-all
just typecheck
just test
just lint
just fmt
just generate
just stack-up
just stack-up-full
just stack-up-observability
just stack-wait
just stack-down
just migrate
just seed
```

Suggested command mapping behind those tasks:

```text
just dev-frontend   -> pnpm turbo run dev --filter=console-web
just dev-ui         -> pnpm turbo run storybook --filter=storybook
just doctor         -> pnpm verify:toolchain
just lint           -> pnpm lint
just typecheck      -> pnpm typecheck
just test           -> pnpm test
just build          -> pnpm build
just generate       -> pnpm generate
just stack-up       -> core Docker Compose services (`postgres`, `redis`, `nats`)
just stack-up-full  -> core services plus the `observability` profile
just stack-wait     -> wait for the selected stack mode to report healthy containers
```

The exact command names can evolve, but the repository should preserve the principle that developers can discover one canonical entry point per workflow.

`pnpm js:test` and `pnpm test` must run `scripts/verify-workspace-tests.mjs` before package tests. Placeholder packages remain listed in `scripts/workspace-test-policy.json` and are reported as explicitly not counted as coverage.

### 29.6 Turbo Configuration Requirements

The initial `turbo.json` should define:

- explicit `outputs`, or explicit empty arrays when a task intentionally emits no stable restorable artifacts yet
- persistent non-cacheable configuration for `dev` and `storybook`
- `dependsOn` edges for buildable internal package relationships
- package-specific overrides only through per-package `turbo.json` files when necessary
- package tags and boundary rules for the first shared frontend workspaces

Do not treat a root `turbo.json` with bare task names as sufficient. Tasks should either declare the artifacts they restore from cache or use `outputs: []` intentionally so Turbo does not warn on every run.

---

## 30. CI/CD

### 30.1 CI Requirements

CI should validate:

- Rust compilation
- frontend type checking
- linting
- unit tests
- contract tests
- generated artifact consistency
- migration validity
- OpenAPI/schema drift
- Storybook or component-library build health for shared UI packages

### 30.2 CD Requirements

Deployment pipelines should support:

- environment promotion
- preview deployments for frontend
- canary releases for gateway services
- migration sequencing
- smoke tests after deployment

### 30.3 Build Caching

Use monorepo-aware caching and selective execution to keep CI efficient.

Recommended baseline:

- `actions/setup-node` `pnpm` cache plus `.turbo` cache for JavaScript jobs
- restorable Turbo task outputs for `dist`, `dist-types`, `.tanstack`, and `storybook-static`
- Cargo registry, Cargo git, and target caching for Rust jobs via `Swatinem/rust-cache`
- filtered execution for `apps/console-web`, `apps/storybook`, and changed shared packages
- remote caching enabled for CI and shared team workflows once the baseline pipeline is green
- advisory repository-boundary validation before hard enforcement

### 30.4 Frontend Pipeline Stages

For the frontend workspace, CI should distinguish at least:

- `lint`
- `typecheck`
- `test`
- `build`
- `storybook` build or smoke verification
- `boundaries` or an equivalent dependency-governance check

This keeps shell regressions, theme breakage, and shared component issues visible before feature branches merge.

Shipped baseline:

- `.github/workflows/quality.yml` runs JavaScript `lint`, `typecheck`, `test`, and `build` as separate jobs on pull requests and pushes to `main`
- `.github/workflows/release.yml` runs on version tags that match `v*`, reruns repository quality gates, generates release notes with `pnpm release:notes`, and publishes a GitHub release
- the same workflow runs Rust `fmt --check`, `clippy`, `check`, and `test` as separate jobs
- JavaScript jobs use `pnpm verify:toolchain` plus the root `js:*` scripts so CI exercises the same command surface used locally
- `pnpm js:test` and `pnpm test` run `scripts/verify-workspace-tests.mjs` before package tests and report the temporary placeholder allowlist from `scripts/workspace-test-policy.json`

### 30.5 Release Channel Policy

- use exact version pinning for TanStack Start while it remains in RC
- allow only intentional, reviewed minor upgrades for Mantine during the bootstrap phase
- record framework upgrades in ADRs
- require changelog review for MCP transport, auth, or framework-level dependency changes

---

## 31. Testing Strategy

### 31.1 Test Layers

1. unit tests
2. integration tests
3. protocol contract tests
4. adapter conformance tests
5. end-to-end tests
6. load tests
7. chaos / resilience tests

### 31.2 Backend Test Focus

- protocol parsing correctness
- route selection correctness
- retry behavior
- metering normalization
- ledger idempotency
- policy enforcement
- degraded-mode behavior
- PI-3 load baselines and deterministic failure scenarios via [`infra/scripts/load-baseline.sh`](../../infra/scripts/load-baseline.sh), [`infra/scripts/failure-injection.sh`](../../infra/scripts/failure-injection.sh), and [`docs/runbooks/load-and-failure-baseline.md`](load-and-failure-baseline.md)

### 31.3 Frontend Test Focus

- route-level auth behavior
- Mantine shell and navigation rendering
- table/filter interactions
- policy editor validation
- optimistic update edge cases
- accessibility for key admin workflows

### 31.4 Component-Library Test Focus

- theme token application
- wrapper compatibility with upstream Mantine changes
- keyboard and focus behavior for shared shell primitives
- loading, empty, and destructive confirmation states

### 31.5 Contract Testing

Every provider adapter should pass a shared conformance suite that verifies:

- capability declaration
- error normalization
- usage extraction
- streaming behavior
- cancellation behavior

---
