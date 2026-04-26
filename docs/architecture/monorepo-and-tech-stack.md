# 5. Monorepo Strategy

[Back to Docs Index](../README.md)

The entire system lives in a single monorepo to optimize for:

- shared schemas and versioning
- coordinated releases
- cross-cutting refactors
- local developer experience
- reproducible CI/CD
- unified testing and contract validation

### 5.1 Why Monorepo

A monorepo is preferred because the system depends on shared protocol definitions, API contracts, UI types, route policy structures, and infrastructure conventions. Fragmenting these into multiple repositories would create unnecessary version skew and significantly increase integration overhead.

### 5.2 Monorepo Constraints

To prevent monorepo sprawl:

- every package must have a clear owner and purpose
- cross-package dependencies must be explicit
- shared packages must remain minimal and stable
- domain boundaries must be reflected in folder structure and CI workflows
- heavy generated assets must not be committed unless required
- frontend packages must not depend on application-only code from `apps/*`
- Rust crates remain the source of truth for backend build graph; JavaScript packages remain the source of truth for frontend build graph
- repository boundaries should be declared early through package tags and ownership rules, even if enforcement starts in advisory mode

### 5.3 Workspace Orchestration Choice

`Turbo` is the required task orchestrator for the JavaScript side of the monorepo.

We are standardizing on `Turbo` because it gives us:

- a simple pipeline model for `build`, `lint`, `typecheck`, `test`, `dev`, and `generate`
- fast incremental execution and cache-aware task reuse in local development and CI
- strong `--filter` ergonomics for working on one app or package without losing monorepo visibility
- straightforward interoperability with `pnpm` workspaces, Storybook, generated clients, and future docs apps
- a lighter operational footprint than `Nx` for the current size and shape of the repository

`Cargo` remains authoritative for Rust compilation and dependency resolution. `Turbo` coordinates repo-level workflows and JavaScript package tasks; it does not replace the Rust workspace.

### 5.4 Developer Workflow Baseline

The intended day-0 workflow is:

- use `pnpm` for JavaScript dependency management
- use `turbo` for frontend and shared-package pipelines
- use `cargo` for Rust compilation, tests, and workspace checks
- use `just` as the top-level command surface that wraps common Rust and Turbo tasks

The standard Turbo pipeline should include at least:

- `dev`
- `build`
- `lint`
- `typecheck`
- `test`
- `storybook`
- `generate`

---

## 6. Proposed Repository Layout

```text
ai-traffic-os/
|-- apps/
|   |-- console-web/                 # TanStack Start app for admin and tenant console
|   |-- docs-site/                   # Optional public docs / marketing / developer docs
|   `-- storybook/                   # Isolated UI component development and review
|-- services/
|   |-- gateway-api/                 # Primary northbound API gateway (Rust)
|   |-- control-plane-api/           # Control plane API (Rust)
|   |-- ledger-worker/               # Usage ledger ingestion and projection worker (Rust)
|   |-- routing-worker/              # Async route scoring / health / optimization worker (Rust)
|   |-- audit-worker/                # Audit event persistence / compliance pipelines (Rust)
|   |-- notification-worker/         # Email/webhook/system notifications (Rust)
|   |-- realtime-gateway/            # Optional dedicated WebSocket / realtime traffic service (Rust)
|   `-- edge-probe/                  # Active health probe / synthetic traffic agent (Rust)
|-- crates/
|   |-- core-domain/                 # Core domain types, IDs, enums, errors, shared traits
|   |-- protocol-ir/                 # Internal representation for AI traffic
|   |-- protocol-openai/             # OpenAI request/response parsing and serialization
|   |-- protocol-anthropic/          # Anthropic Messages protocol implementation
|   |-- protocol-gemini/             # Gemini protocol implementation
|   |-- protocol-realtime/           # Realtime event protocol handling
|   |-- provider-traits/             # Adapter traits for upstream providers/gateways
|   |-- plugin-sdk/                  # Stable extension contracts, manifests, and capability descriptors
|   |-- plugin-registry/             # Registration, discovery, and runtime lookup of pluggable modules
|   |-- runtime-composition/         # Composition root, middleware chains, and pipeline assembly
|   |-- provider-openai/             # OpenAI upstream adapter
|   |-- provider-anthropic/          # Anthropic upstream adapter
|   |-- provider-gemini/             # Gemini upstream adapter
|   |-- provider-gateway/            # Adapter for upstream transit systems/gateways
|   |-- routing-engine/              # Core route evaluation engine
|   |-- policy-engine/               # Policy evaluation and enforcement
|   |-- authn-authz/                 # API key, JWT, RBAC, tenant scope, mTLS helpers
|   |-- metering/                    # Usage extraction, normalization, unit accounting
|   |-- ledger-models/               # Ledger events, projections, balance models
|   |-- storage/                     # DB repositories, query abstractions, migrations glue
|   |-- cache/                       # Redis/cache abstractions and degraded-mode logic
|   |-- telemetry/                   # OpenTelemetry helpers, tracing, structured logging
|   |-- queue/                       # Event bus abstractions
|   |-- config/                      # Shared configuration loader/validation
|   |-- sdk-server/                  # Internal shared server utilities
|   `-- testing-kit/                 # Test harnesses, fixtures, mocks, contract runners
|-- packages/
|   |-- ts-api-client/               # Generated TypeScript client for control plane API
|   |-- ts-shared-schema/            # Shared Zod/TypeScript schemas for frontend
|   |-- typescript-config/           # Shared tsconfig presets
|   |-- ui-kit/                      # HeroUI-wrapped shared React component library
|   |-- design-tokens/               # HeroUI theme, semantic tokens, typography, spacing
|   |-- docs-content/                # Markdown/MDX docs content
|   `-- test-utils/                  # Frontend testing helpers
|-- infra/
|   |-- docker/                      # Dockerfiles and compose definitions
|   |-- k8s/                         # Kubernetes manifests / Helm charts
|   |-- terraform/                   # Cloud provisioning
|   |-- monitoring/                  # Dashboards / alerts / collector configs
|   `-- scripts/                     # Infra automation scripts
|-- schemas/
|   |-- openapi/                     # OpenAPI specs for control plane and public APIs
|   |-- jsonschema/                  # JSON schemas for configs and policy docs
|   `-- examples/                    # Example requests/responses
|-- docs/
|   |-- architecture/
|   |-- adr/
|   |-- runbooks/
|   |-- api/
|   `-- product/
|-- .github/
|   `-- workflows/
|-- Cargo.toml                       # Rust workspace root
|-- package.json                     # JS workspace root
|-- pnpm-workspace.yaml              # JS workspace configuration
|-- turbo.json                       # Required JavaScript task orchestration
|-- justfile                         # Developer task runner
|-- rust-toolchain.toml              # Toolchain pinning
`-- README.md
```

---

## 7. Technology Choices

### 7.1 Backend

- **Language:** Rust
- **Edition:** Stable Rust, pinned via `rust-toolchain.toml`
- **Primary web framework:** Axum `0.8.x` (current stable in this repository baseline, Tower-native middleware)
- **Async runtime:** Tokio `1.x` (current stable line in this repository baseline)
- **Serialization:** Serde
- **Error handling:** `thiserror` + `anyhow` at service boundaries where appropriate
- **Validation:** validator or custom domain validation layer
- **HTTP client:** `reqwest` + hyper ecosystem where needed
- **Database:** PostgreSQL
- **Cache / counters / lease storage:** Redis
- **Message bus:** NATS JetStream or Kafka
- **Metrics / tracing:** OpenTelemetry + Prometheus
- **Structured logs:** `tracing` + JSON log output
- **Policy evaluation:** OPA sidecar or embedded policy engine abstraction
- **Migrations:** SQLx migrations or refinery

### 7.2 Frontend

- **App framework:** TanStack Start
- **Language:** TypeScript
- **Routing:** TanStack Router
- **Data fetching / caching:** TanStack Query
- **Tables:** TanStack Table
- **Forms:** TanStack Form + Zod validation for the primary path
- **Schema validation:** Zod
- **Component foundation:** HeroUI
- **App styling:** HeroUI theme + Styles API, with CSS Modules for app-specific layout and feature styling
- **State management:** local-first plus TanStack Query; avoid unnecessary global stores
- **Charts:** Recharts or similar lightweight charting
- **Notifications / overlays:** HeroUI toast/feedback surfaces and project-owned overlay wrappers
- **Testing:** Vitest + Playwright

### 7.3 Frontend Package Responsibilities

- `apps/console-web` owns route composition, loaders, mutations, feature modules, and auth-aware layouts
- `packages/design-tokens` owns HeroUI theme creation, semantic colors, spacing, radius, shadows, and typography decisions
- `packages/ui-kit` owns reusable HeroUI-based primitives and higher-level console components
- `apps/storybook` is the review surface for `packages/ui-kit` and shell primitives before feature integration

### 7.4 Developer Tooling

- **Frontend runtime baseline:** Node.js 24 LTS in local development and CI
- **JS package manager:** pnpm
- **Task orchestration:** Turbo
- **Task runner:** just
- **Linting:** Clippy for Rust, Oxlint for TypeScript
- **Formatting:** rustfmt + Prettier
- **Commit hooks:** lefthook or Husky
- **Code generation:** OpenAPI-based TS client generation, JSON Schema generation, Rust-to-TS schema generation where applicable
- **CI caching:** Turbo cache for JavaScript tasks, Cargo registry/target caching for Rust, and shared artifact caching where safe

### 7.5 Composition Model

- **Plugin model:** compile-time contracts and config-driven registration first
- **Service assembly:** explicit composition roots per service
- **Extension discovery:** manifest- and capability-based registry lookup
- **Cross-cutting concerns:** middleware/pipeline composition rather than ad hoc branching in service handlers

### 7.6 Version Channel Policy

As of **April 20, 2026**, the repository frontend baseline is:

- Node.js `24.x` LTS for local development and CI
- React `19.2+`
- HeroUI `9.0.x`
- TanStack Start `1.x`

Implementation policy:

- keep TanStack Start, TanStack Router, and closely coupled packages pinned to exact versions in the repository until upgrade cadence, compatibility expectations, and lockfile review discipline are proven in CI
- pin HeroUI to the `9.0.x` minor line during initial implementation and upgrade intentionally with changelog review
- commit `packageManager` in the root `package.json` and use Corepack in CI
- treat framework major upgrades as explicit architecture changes, not routine dependency bumps

As of **April 20, 2026**, the repository backend baseline is:

- Axum `0.8.x`
- Tokio `1.x`
- Rust stable, pinned via `rust-toolchain.toml`

### 7.7 Monorepo Task Policy

- define `outputs` for every cacheable Turbo task from day one
- use per-package `turbo.json` package configurations only where a package genuinely needs overrides
- enable remote caching for CI and shared team workflows after the first stable build pipeline lands
- keep long-running tasks such as `dev` and `storybook` marked as non-cacheable persistent tasks
- never rely on implicit task ordering when a dependency edge or transit node is required

### 7.8 Repository Boundary Policy

To keep the monorepo composable as teams begin parallel work:

- classify JavaScript workspaces with tags such as `app`, `ui-public`, `ui-internal`, `frontend-contract`, and `tooling`
- keep `packages/*` free of imports from `apps/*`
- use per-package `turbo.json` files to declare tags or narrowly scoped overrides instead of overloading the root config
- introduce `turbo boundaries` or an equivalent dependency check early in CI as an advisory signal
- promote boundary checks to blocking only after the initial package layout stabilizes

This gives the repository enough governance to scale without freezing early scaffolding work.

---
