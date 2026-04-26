# Task Catalog

[Back to Task Catalog Index](README.md)

| Task ID | Phase | Stream                             | Size | Depends On                                           | Title                                                                        |
| ------- | ----- | ---------------------------------- | ---- | ---------------------------------------------------- | ---------------------------------------------------------------------------- |
| FND-001 | PI-0  | Foundation and Monorepo            | L    | None                                                 | Initialize Rust workspace and service skeletons                              |
| FND-002 | PI-0  | Foundation and Monorepo            | M    | None                                                 | Initialize pnpm + Turbo workspace and TanStack Start application shell       |
| FND-003 | PI-0  | Foundation and Monorepo            | M    | None                                                 | Provision local development stack and containerized runtime                  |
| FND-004 | PI-0  | Foundation and Monorepo            | M    | FND-001                                              | Establish shared configuration, secrets, and environment loading             |
| FND-005 | PI-0  | Foundation and Monorepo            | M    | FND-001, FND-002                                     | Set up CI pipelines, quality gates, and caching strategy                     |
| FND-006 | PI-0  | Foundation and Monorepo            | M    | FND-002                                              | Create schema pipeline for OpenAPI, JSON Schema, and generated TS client     |
| GWT-001 | PI-1  | Gateway and Protocol Ingress       | M    | FND-001, FND-003, FND-004                            | Implement gateway HTTP server skeleton and middleware chain                  |
| GWT-002 | PI-1  | Gateway and Protocol Ingress       | M    | CTL-001, SEC-001                                     | Implement API key authentication and tenant/project resolution               |
| GWT-003 | PI-1  | Gateway and Protocol Ingress       | L    | FND-001                                              | Define protocol IR v1 with request, response, and capability models          |
| GWT-004 | PI-1  | Gateway and Protocol Ingress       | L    | GWT-001, GWT-003, PAD-001, PAD-002, RTE-002, MET-001 | Ship OpenAI-compatible northbound chat/completions ingress                   |
| GWT-005 | PI-1  | Gateway and Protocol Ingress       | M    | GWT-004                                              | Implement SSE streaming relay and backpressure-safe response path            |
| GWT-006 | PI-2  | Gateway and Protocol Ingress       | M    | GWT-003, PAD-003, RTE-002                            | Add Anthropic-native northbound protocol support                             |
| GWT-007 | PI-2  | Gateway and Protocol Ingress       | M    | GWT-003, PAD-004, RTE-002                            | Add Gemini-native northbound protocol support                                |
| GWT-008 | PI-4  | Gateway and Protocol Ingress       | L    | FND-001, OBS-001, RTE-004                            | Build realtime session gateway skeleton                                      |
| CTL-001 | PI-0  | Control Plane and Console          | M    | FND-001, FND-004                                     | Create control plane API service skeleton with typed CRUD conventions        |
| CTL-002 | PI-1  | Control Plane and Console          | L    | CTL-001, DB-001, SEC-001                             | Implement tenant, project, environment, and API key CRUD                     |
| CTL-003 | PI-1  | Control Plane and Console          | L    | CTL-001, PAD-001, RTE-001, SEC-003                   | Implement provider, credential, route policy, and route set management APIs  |
| CTL-004 | PI-0  | Control Plane and Console          | M    | FND-002, FND-006                                     | Build TanStack Start + HeroUI application shell and authenticated app layout |
| CTL-005 | PI-1  | Control Plane and Console          | M    | CTL-002, CTL-004                                     | Implement tenant/project management UI flows                                 |
| CTL-006 | PI-2  | Control Plane and Console          | L    | CTL-003, CTL-004, RTE-005                            | Implement provider and route management UI with diagnostics surfaces         |
| CTL-007 | PI-3  | Control Plane and Console          | L    | MET-004, MET-005, CTL-004                            | Build usage, budget, and billing dashboards                                  |
| DB-001  | PI-0  | Data Storage and Events            | L    | FND-001, FND-003                                     | Design relational schema, migrations, and repository primitives              |
| DB-002  | PI-0  | Data Storage and Events            | M    | FND-001, FND-003                                     | Implement event bus abstractions and NATS-backed publishers/consumers        |
| PAD-001 | PI-1  | Provider Adapters                  | M    | GWT-003, FND-001                                     | Define provider adapter trait model and adapter conformance test kit         |
| PAD-002 | PI-1  | Provider Adapters                  | M    | PAD-001, GWT-003                                     | Implement OpenAI upstream adapter                                            |
| PAD-003 | PI-2  | Provider Adapters                  | M    | PAD-001, GWT-003                                     | Implement Anthropic upstream adapter                                         |
| PAD-004 | PI-2  | Provider Adapters                  | M    | PAD-001, GWT-003                                     | Implement Gemini upstream adapter                                            |
| PAD-005 | PI-4  | Provider Adapters                  | M    | PAD-001, RTE-004                                     | Implement gateway-of-gateways adapter for upstream transit systems           |
| RTE-001 | PI-1  | Routing and Reliability            | M    | GWT-003, DB-001                                      | Implement route, provider target, and policy domain models                   |
| RTE-002 | PI-1  | Routing and Reliability            | L    | RTE-001, PAD-001                                     | Implement routing engine MVP for static selection and fallback               |
| RTE-003 | PI-2  | Routing and Reliability            | M    | RTE-001, DB-002, OBS-001                             | Implement active health probes and passive health aggregation                |
| RTE-004 | PI-2  | Routing and Reliability            | L    | RTE-002, RTE-003, OBS-001                            | Implement retry budgets, circuit breaking, and quarantine logic              |
| RTE-005 | PI-2  | Routing and Reliability            | M    | RTE-002, RTE-003, CTL-001                            | Expose route diagnostics APIs and explanation model                          |
| MET-001 | PI-1  | Metering, Ledger, and Billing      | M    | GWT-003, DB-002                                      | Define usage event schema and hot-path usage extraction hooks                |
| MET-002 | PI-2  | Metering, Ledger, and Billing      | L    | MET-001, DB-001, DB-002                              | Build ledger worker for immutable event ingestion and deduplication          |
| MET-003 | PI-3  | Metering, Ledger, and Billing      | L    | MET-002, CTL-003                                     | Implement pricing engine with token, cached-token, and multimodal units      |
| MET-004 | PI-3  | Metering, Ledger, and Billing      | M    | MET-002, MET-003                                     | Build balance, budget, and projection read models                            |
| MET-005 | PI-3  | Metering, Ledger, and Billing      | M    | MET-004, CTL-001                                     | Implement billing and usage analytics endpoints                              |
| SEC-001 | PI-0  | Security, Identity, and Compliance | M    | FND-001, CTL-001                                     | Define RBAC model, scopes, and authorization middleware                      |
| SEC-002 | PI-3  | Security, Identity, and Compliance | M    | CTL-004, SEC-001                                     | Implement OIDC/SSO login flow for console users                              |
| SEC-003 | PI-1  | Security, Identity, and Compliance | M    | FND-004, DB-001                                      | Implement secret reference model and provider credential storage boundary    |
| SEC-004 | PI-2  | Security, Identity, and Compliance | M    | DB-002, SEC-001, OBS-001                             | Build audit event pipeline and retention controls                            |
| OBS-001 | PI-0  | Observability, SRE, and Runtime    | M    | FND-001, FND-003                                     | Implement telemetry crate and platform-wide OpenTelemetry conventions        |
| OBS-002 | PI-1  | Observability, SRE, and Runtime    | M    | OBS-001, GWT-001, CTL-001                            | Build service-level dashboards and SLO definitions                           |
| OBS-003 | PI-2  | Observability, SRE, and Runtime    | S    | OBS-002, SEC-004                                     | Implement alert routing and incident notification pipeline                   |
| OBS-004 | PI-3  | Observability, SRE, and Runtime    | L    | GWT-005, RTE-004, MET-002                            | Create load, soak, and failure-injection test suite                          |
| QAR-001 | PI-1  | QA, Release, and Documentation     | M    | GWT-004, PAD-002, FND-006                            | Build protocol contract tests and golden fixtures                            |
| QAR-002 | PI-2  | QA, Release, and Documentation     | M    | CTL-005, GWT-005, MET-002                            | Create end-to-end integration environment and seeded demo tenant             |
| QAR-003 | PI-2  | QA, Release, and Documentation     | M    | FND-005                                              | Implement release automation, versioning policy, and changelog generation    |
| QAR-004 | PI-0  | QA, Release, and Documentation     | S    | None                                                 | Create agent-facing implementation guides and definition-of-done checklists  |

## Detailed records

### FND-001 - Initialize Rust workspace and service skeletons

- **Phase:** PI-0
- **Stream:** Foundation and Monorepo
- **Size:** L
- **Depends on:** None
- **Primary paths:** `Cargo.toml`, `crates/*`, `services/*`, `justfile`
- **Expected outputs:**
  - Rust workspace root
  - service crate templates
  - shared lint/test commands
- **Acceptance criteria:**
  - `cargo check --workspace` passes
  - all services compile as placeholder binaries
  - workspace dependency policy is documented

### FND-002 - Initialize pnpm + Turbo workspace and TanStack Start application shell

- **Phase:** PI-0
- **Stream:** Foundation and Monorepo
- **Size:** M
- **Depends on:** None
- **Primary paths:** `package.json`, `pnpm-workspace.yaml`, `turbo.json`, `apps/console-web`, `apps/storybook`, `packages/ui-kit`, `packages/design-tokens`, `packages/typescript-config`, `apps/console-web/src/start.ts`, `apps/*/turbo.json`, `packages/*/turbo.json`
- **Expected outputs:**
  - pnpm workspace root
  - Turbo pipeline baseline
  - TanStack Start app shell
  - HeroUI provider and theme bootstrap
  - TanStack Start global middleware bootstrap
  - typed route skeleton
  - shared TS config
  - initial workspace tags and boundary-ready package configurations
- **Acceptance criteria:**
  - `pnpm install`, `pnpm turbo run dev --filter=console-web`, and `pnpm turbo run typecheck --filter=console-web` work
  - React 19.2+ and HeroUI 3 baseline are pinned and compatible
  - TanStack Start RC version is pinned exactly
  - root layout, auth placeholder, HeroUI theme provider, TanStack Start global middleware, and basic navigation render
  - core JavaScript workspaces declare package tags or equivalent ownership metadata
  - shared UI primitives can render in Storybook or equivalent isolated component sandbox
  - build passes in CI

### FND-003 - Provision local development stack and containerized runtime

- **Phase:** PI-0
- **Stream:** Foundation and Monorepo
- **Size:** M
- **Depends on:** None
- **Primary paths:** `infra/docker`, `infra/scripts`, `.env.example`
- **Expected outputs:**
  - Docker Compose stack
  - local Postgres/Redis/NATS/OTel collectors
  - bootstrap scripts
- **Acceptance criteria:**
  - one command starts local dependencies
  - health checks expose readiness
  - local stack is documented in runbook

### FND-004 - Establish shared configuration, secrets, and environment loading

- **Phase:** PI-0
- **Stream:** Foundation and Monorepo
- **Size:** M
- **Depends on:** FND-001
- **Primary paths:** `crates/config`, `crates/sdk-server`, `docs/runbooks`
- **Expected outputs:**
  - typed config crate
  - env validation
  - secret provider interface
- **Acceptance criteria:**
  - all services load validated config
  - missing required variables fail fast
  - secret-backed values are separated from non-secret config

### FND-005 - Set up CI pipelines, quality gates, and caching strategy

- **Phase:** PI-0
- **Stream:** Foundation and Monorepo
- **Size:** M
- **Depends on:** FND-001, FND-002
- **Primary paths:** `.github/workflows`, `turbo.json`, `package.json`, `justfile`
- **Expected outputs:**
  - CI workflows
  - Turbo task graph
  - selective change detection
  - artifact caching
  - remote caching rollout plan
  - repository boundary validation rollout
- **Acceptance criteria:**
  - pull requests run Rust and TS checks
  - jobs are split by workspace scope and use Turbo filters where appropriate
  - cacheable Turbo tasks declare `outputs`
  - advisory repository boundary checks run in CI for tagged workspaces
  - cache hit rate is measurable

### FND-006 - Create schema pipeline for OpenAPI, JSON Schema, and generated TS client

- **Phase:** PI-0
- **Stream:** Foundation and Monorepo
- **Size:** M
- **Depends on:** FND-002
- **Primary paths:** `schemas/openapi`, `schemas/jsonschema`, `packages/ts-api-client`, `packages/ts-shared-schema`
- **Expected outputs:**
  - schema source-of-truth workflow
  - TS client generation
  - schema linting
- **Acceptance criteria:**
  - OpenAPI documents validate
  - TS client can be regenerated from CI
  - console consumes generated types

### GWT-001 - Implement gateway HTTP server skeleton and middleware chain

- **Phase:** PI-1
- **Stream:** Gateway and Protocol Ingress
- **Size:** M
- **Depends on:** FND-001, FND-003, FND-004
- **Primary paths:** `services/gateway-api`, `crates/sdk-server`, `crates/telemetry`
- **Expected outputs:**
  - HTTP server bootstrap
  - middleware pipeline
  - health/readiness endpoints
- **Acceptance criteria:**
  - service starts with trace and metrics middleware
  - health endpoints are covered by tests
  - configuration is hot-reload safe or explicitly immutable

### GWT-002 - Implement API key authentication and tenant/project resolution

- **Phase:** PI-1
- **Stream:** Gateway and Protocol Ingress
- **Size:** M
- **Depends on:** CTL-001, SEC-001
- **Primary paths:** `services/gateway-api`, `crates/authn-authz`, `crates/storage`
- **Expected outputs:**
  - API key auth middleware
  - tenant/project context injection
  - error model
- **Acceptance criteria:**
  - valid keys resolve tenant and project scopes
  - invalid/disabled keys fail with normalized errors
  - auth path emits audit and trace metadata

### GWT-003 - Define protocol IR v1 with request, response, and capability models

- **Phase:** PI-1
- **Stream:** Gateway and Protocol Ingress
- **Size:** L
- **Depends on:** FND-001
- **Primary paths:** `crates/protocol-ir`, `crates/core-domain`, `docs/architecture/protocol-ir-and-protocols.md`
- **Expected outputs:**
  - IR data structures
  - semantic validation
  - provider extension fields
  - extension envelope rules
- **Acceptance criteria:**
  - IR covers text generation MVP
  - unknown provider-specific metadata can be preserved
  - extension fields are namespaced and versionable
  - IR versioning strategy is documented

### GWT-004 - Ship OpenAI-compatible northbound chat/completions ingress

- **Phase:** PI-1
- **Stream:** Gateway and Protocol Ingress
- **Size:** L
- **Depends on:** GWT-001, GWT-003, PAD-001, PAD-002, RTE-002, MET-001
- **Primary paths:** `services/gateway-api`, `crates/protocol-openai`, `schemas/examples`
- **Expected outputs:**
  - OpenAI-compatible endpoint
  - request parsing to IR
  - response mapping from IR/provider result
- **Acceptance criteria:**
  - non-streaming chat requests work end-to-end
  - OpenAI-compatible errors are returned
  - golden fixture tests pass

### GWT-005 - Implement SSE streaming relay and backpressure-safe response path

- **Phase:** PI-1
- **Stream:** Gateway and Protocol Ingress
- **Size:** M
- **Depends on:** GWT-004
- **Primary paths:** `services/gateway-api`, `crates/sdk-server`, `crates/protocol-openai`
- **Expected outputs:**
  - stream relay
  - disconnect handling
  - partial usage capture hooks
- **Acceptance criteria:**
  - streaming responses are flushed incrementally
  - client disconnects cancel upstream requests
  - stream tests cover partial and terminal events

### GWT-006 - Add Anthropic-native northbound protocol support

- **Phase:** PI-2
- **Stream:** Gateway and Protocol Ingress
- **Size:** M
- **Depends on:** GWT-003, PAD-003, RTE-002
- **Primary paths:** `crates/protocol-anthropic`, `services/gateway-api`
- **Expected outputs:**
  - Anthropic request/response mapping
  - protocol-specific validation
- **Acceptance criteria:**
  - Anthropic-native message requests reach supported upstreams
  - provider-specific fields are preserved when possible
  - contract tests pass

### GWT-007 - Add Gemini-native northbound protocol support

- **Phase:** PI-2
- **Stream:** Gateway and Protocol Ingress
- **Size:** M
- **Depends on:** GWT-003, PAD-004, RTE-002
- **Primary paths:** `crates/protocol-gemini`, `services/gateway-api`
- **Expected outputs:**
  - Gemini request/response mapping
  - tool/function compatibility notes
- **Acceptance criteria:**
  - Gemini text generation works end-to-end
  - unsupported constructs fail with explicit compatibility errors
  - fixtures cover mapping edge cases

### GWT-008 - Build realtime session gateway skeleton

- **Phase:** PI-4
- **Stream:** Gateway and Protocol Ingress
- **Size:** L
- **Depends on:** FND-001, OBS-001, RTE-004
- **Primary paths:** `services/realtime-gateway`, `crates/protocol-realtime`, `crates/authn-authz`
- **Expected outputs:**
  - WebSocket service skeleton
  - session auth
  - realtime connection model
- **Acceptance criteria:**
  - service can accept authenticated sessions
  - session lifecycle is traced
  - no billing is coupled to the connection handshake

### CTL-001 - Create control plane API service skeleton with typed CRUD conventions

- **Phase:** PI-0
- **Stream:** Control Plane and Console
- **Size:** M
- **Depends on:** FND-001, FND-004
- **Primary paths:** `services/control-plane-api`, `crates/sdk-server`, `schemas/openapi`
- **Expected outputs:**
  - service bootstrap
  - error envelope
  - pagination/filtering conventions
- **Acceptance criteria:**
  - service exposes versioned base path
  - OpenAPI spec is generated from implementation
  - integration test can start service against local stack

### CTL-002 - Implement tenant, project, environment, and API key CRUD

- **Phase:** PI-1
- **Stream:** Control Plane and Console
- **Size:** L
- **Depends on:** CTL-001, DB-001, SEC-001
- **Primary paths:** `services/control-plane-api`, `crates/storage`, `crates/core-domain`
- **Expected outputs:**
  - resource CRUD endpoints
  - API key issuance/rotation
  - resource validation
- **Acceptance criteria:**
  - CRUD endpoints are idempotent where expected
  - API key rotation invalidates old secrets
  - OpenAPI and TS client are updated

### CTL-003 - Implement provider, credential, route policy, and route set management APIs

- **Phase:** PI-1
- **Stream:** Control Plane and Console
- **Size:** L
- **Depends on:** CTL-001, PAD-001, RTE-001, SEC-003
- **Primary paths:** `services/control-plane-api`, `crates/storage`, `crates/provider-traits`, `crates/routing-engine`
- **Expected outputs:**
  - provider CRUD
  - credential binding
  - route policy CRUD
  - provider-resource support for official APIs and third-party relay or gateway endpoints
- **Acceptance criteria:**
  - route policies can be created and validated from API
  - secret references are stored without plaintext leakage
  - policy schema validation errors are actionable
  - provider resources can model relay flavor, auth strategy, and required header passthrough without free-form per-site hacks

### CTL-004 - Build TanStack Start + HeroUI application shell and authenticated app layout

- **Phase:** PI-0
- **Stream:** Control Plane and Console
- **Size:** M
- **Depends on:** FND-002, FND-006
- **Primary paths:** `apps/console-web`, `packages/ui-kit`, `packages/design-tokens`
- **Expected outputs:**
  - route tree
  - shell layout
  - HeroUI-backed theme and navigation system
  - shared provider stack and shell primitives
- **Acceptance criteria:**
  - authenticated and unauthenticated layouts render
  - internal HeroUI-compatible shell or equivalent shell wrapper is in place
  - route-level data loaders compile
  - component story coverage exists for shell primitives

### CTL-005 - Implement tenant/project management UI flows

- **Phase:** PI-1
- **Stream:** Control Plane and Console
- **Size:** M
- **Depends on:** CTL-002, CTL-004
- **Primary paths:** `apps/console-web/src/routes`, `packages/ts-api-client`
- **Expected outputs:**
  - list/detail/create flows
  - API key issuance views
  - error/loading states
- **Acceptance criteria:**
  - CRUD flows are fully typed from generated client
  - forms validate client-side and server-side
  - role-based page guards work

### CTL-006 - Implement provider and route management UI with diagnostics surfaces

- **Phase:** PI-2
- **Stream:** Control Plane and Console
- **Size:** L
- **Depends on:** CTL-003, CTL-004, RTE-005
- **Primary paths:** `apps/console-web/src/routes/providers`, `apps/console-web/src/routes/routes`
- **Expected outputs:**
  - provider detail pages
  - route editors
  - route diagnostics panels
  - relay-aware provider resource forms and validation flows
  - compatibility-profile selection and preset validation for common relay families
- **Acceptance criteria:**
  - operators can inspect route health and policy resolution
  - form edits are optimistic only where safe
  - dangerous operations require confirmation
  - operators can create and manage third-party relay resources by endpoint and key across multiple compatibility families, including One API-like, New API-like, Sub2API-like, LiteLLM-like, LMRouter-like, and generic OpenAI-compatible gateways

### CTL-007 - Build usage, budget, and billing dashboards

- **Phase:** PI-3
- **Stream:** Control Plane and Console
- **Size:** L
- **Depends on:** MET-004, MET-005, CTL-004
- **Primary paths:** `apps/console-web/src/routes/usage`, `apps/console-web/src/routes/billing`
- **Expected outputs:**
  - usage charts
  - budget threshold views
  - balance and invoice projections
  - payment initiation and status surfaces for the first billing collection flow
- **Acceptance criteria:**
  - dashboards match projection APIs
  - time-range filters and tenant scopes work
  - large tables are paginated and exportable
  - the first payment flow exposed in the console supports WeChat Pay only and states that scope clearly

### DB-001 - Design relational schema, migrations, and repository primitives

- **Phase:** PI-0
- **Stream:** Data Storage and Events
- **Size:** L
- **Depends on:** FND-001, FND-003
- **Primary paths:** `crates/storage`, `infra/docker`, `docs/architecture/data-storage-and-events.md`
- **Expected outputs:**
  - migration framework
  - core tables
  - repository abstractions
- **Acceptance criteria:**
  - fresh database bootstraps successfully
  - rollback policy is documented
  - repositories are integration-tested

### DB-002 - Implement event bus abstractions and NATS-backed publishers/consumers

- **Phase:** PI-0
- **Stream:** Data Storage and Events
- **Size:** M
- **Depends on:** FND-001, FND-003
- **Primary paths:** `crates/queue`, `services/*`, `crates/sdk-server`
- **Expected outputs:**
  - event publishing API
  - consumer worker harness
  - subject naming conventions
- **Acceptance criteria:**
  - workers can consume and ack events
  - message schemas are versioned
  - dead-letter strategy is defined

### PAD-001 - Define provider adapter trait model and adapter conformance test kit

- **Phase:** PI-1
- **Stream:** Provider Adapters
- **Size:** M
- **Depends on:** GWT-003, FND-001
- **Primary paths:** `crates/provider-traits`, `crates/testing-kit`, `docs/architecture/provider-adapter-system.md`
- **Expected outputs:**
  - adapter trait set
  - adapter manifest and registry model
  - error normalization model
  - conformance fixtures
- **Acceptance criteria:**
  - all adapters implement shared trait surfaces
  - registry can resolve adapters by provider kind and capability profile
  - adapter tests can run against mocks
  - error categories map to routing decisions
  - manifests can declare relay-specific compatibility details such as gateway flavor, header strategy, and sticky-session passthrough requirements
  - relay compatibility profiles are extensible enough to cover One API-like, New API-like, Sub2API-like, LiteLLM-like, LMRouter-like, and generic OpenAI-compatible upstreams

### PAD-002 - Implement OpenAI upstream adapter

- **Phase:** PI-1
- **Stream:** Provider Adapters
- **Size:** M
- **Depends on:** PAD-001, GWT-003
- **Primary paths:** `crates/provider-openai`
- **Expected outputs:**
  - OpenAI provider client
  - streaming and non-streaming calls
  - usage extraction hooks
- **Acceptance criteria:**
  - adapter passes conformance fixtures
  - timeouts and retries are configurable
  - provider errors map to normalized categories

### PAD-003 - Implement Anthropic upstream adapter

- **Phase:** PI-2
- **Stream:** Provider Adapters
- **Size:** M
- **Depends on:** PAD-001, GWT-003
- **Primary paths:** `crates/provider-anthropic`
- **Expected outputs:**
  - Anthropic provider client
  - message API integration
  - usage extraction hooks
- **Acceptance criteria:**
  - adapter passes conformance fixtures
  - unsupported native features are surfaced clearly
  - timeout and region configs are supported

### PAD-004 - Implement Gemini upstream adapter

- **Phase:** PI-2
- **Stream:** Provider Adapters
- **Size:** M
- **Depends on:** PAD-001, GWT-003
- **Primary paths:** `crates/provider-gemini`
- **Expected outputs:**
  - Gemini provider client
  - compatibility layer
  - usage extraction hooks
- **Acceptance criteria:**
  - adapter passes conformance fixtures
  - model capability metadata is discoverable
  - error normalization is documented

### PAD-005 - Implement gateway-of-gateways adapter for upstream transit systems

- **Phase:** PI-4
- **Stream:** Provider Adapters
- **Size:** M
- **Depends on:** PAD-001, RTE-004
- **Primary paths:** `crates/provider-gateway`
- **Expected outputs:**
  - adapter for upstream gateway providers
  - health and capability metadata model
  - compatibility profiles for common relay and broker families
- **Acceptance criteria:**
  - adapter can call an upstream OpenAI-compatible gateway
  - upstream gateway errors preserve diagnostic detail
  - routing engine can score it alongside native providers
  - adapter can authenticate against third-party relay keys and preserve declared relay-required headers without leaking them into unrelated providers
  - profile coverage includes at least generic OpenAI-compatible gateways plus documented family-specific handling for One API-like, New API-like, Sub2API-like, LiteLLM-like, and LMRouter-like upstreams where needed

### RTE-001 - Implement route, provider target, and policy domain models

- **Phase:** PI-1
- **Stream:** Routing and Reliability
- **Size:** M
- **Depends on:** GWT-003, DB-001
- **Primary paths:** `crates/routing-engine`, `crates/core-domain`, `services/control-plane-api`
- **Expected outputs:**
  - route entities
  - policy model
  - score input structures
- **Acceptance criteria:**
  - route entities support weighted and ordered strategies
  - validation prevents impossible configurations
  - models are reusable by API and workers

### RTE-002 - Implement routing engine MVP for static selection and fallback

- **Phase:** PI-1
- **Stream:** Routing and Reliability
- **Size:** L
- **Depends on:** RTE-001, PAD-001
- **Primary paths:** `crates/routing-engine`, `services/gateway-api`
- **Expected outputs:**
  - route evaluator
  - fallback chain execution
  - selection reasoning output
- **Acceptance criteria:**
  - engine selects eligible route deterministically
  - fallback occurs on retryable error classes
  - selection reason is emitted in traces

### RTE-003 - Implement active health probes and passive health aggregation

- **Phase:** PI-2
- **Stream:** Routing and Reliability
- **Size:** M
- **Depends on:** RTE-001, DB-002, OBS-001
- **Primary paths:** `services/edge-probe`, `services/routing-worker`, `crates/routing-engine`
- **Expected outputs:**
  - probe scheduler
  - health score aggregation
  - route health persistence
- **Acceptance criteria:**
  - probe results update route health state
  - cheap and billable probe modes are separated
  - quarantine thresholds are configurable

### RTE-004 - Implement retry budgets, circuit breaking, and quarantine logic

- **Phase:** PI-2
- **Stream:** Routing and Reliability
- **Size:** L
- **Depends on:** RTE-002, RTE-003, OBS-001
- **Primary paths:** `crates/routing-engine`, `services/gateway-api`, `services/routing-worker`
- **Expected outputs:**
  - retry budget policy
  - circuit breaker state
  - quarantine transitions
- **Acceptance criteria:**
  - retry loops are bounded and observable
  - bad routes can be quarantined automatically
  - manual override APIs exist

### RTE-005 - Expose route diagnostics APIs and explanation model

- **Phase:** PI-2
- **Stream:** Routing and Reliability
- **Size:** M
- **Depends on:** RTE-002, RTE-003, CTL-001
- **Primary paths:** `services/control-plane-api`, `crates/routing-engine`, `schemas/openapi`
- **Expected outputs:**
  - diagnostics endpoints
  - route explanation model
  - recent failure summaries
- **Acceptance criteria:**
  - operators can query why a route was or was not selected
  - health and score inputs are visible with redaction
  - OpenAPI docs include example responses

### MET-001 - Define usage event schema and hot-path usage extraction hooks

- **Phase:** PI-1
- **Stream:** Metering, Ledger, and Billing
- **Size:** M
- **Depends on:** GWT-003, DB-002
- **Primary paths:** `crates/metering`, `crates/ledger-models`, `services/gateway-api`
- **Expected outputs:**
  - usage event schema
  - gateway emission hooks
  - idempotency key format
- **Acceptance criteria:**
  - usage events can be emitted for success and failure paths
  - schema version is explicit
  - events include tenant/project/route correlation IDs

### MET-002 - Build ledger worker for immutable event ingestion and deduplication

- **Phase:** PI-2
- **Stream:** Metering, Ledger, and Billing
- **Size:** L
- **Depends on:** MET-001, DB-001, DB-002
- **Primary paths:** `services/ledger-worker`, `crates/ledger-models`, `crates/storage`
- **Expected outputs:**
  - immutable ledger persistence
  - deduplication logic
  - repair/replay hooks
- **Acceptance criteria:**
  - duplicate usage events are ignored safely
  - ledger writes are append-only
  - replay against a snapshot produces deterministic results

### MET-003 - Implement pricing engine with token, cached-token, and multimodal units

- **Phase:** PI-3
- **Stream:** Metering, Ledger, and Billing
- **Size:** L
- **Depends on:** MET-002, CTL-003
- **Primary paths:** `crates/metering`, `crates/ledger-models`, `services/control-plane-api`
- **Expected outputs:**
  - pricing rules engine
  - price table model
  - cost calculation library
- **Acceptance criteria:**
  - pricing can model input/output/cache/image/audio units
  - pricing rules are versioned and auditable
  - simulation endpoint exists for test scenarios

### MET-004 - Build balance, budget, and projection read models

- **Phase:** PI-3
- **Stream:** Metering, Ledger, and Billing
- **Size:** M
- **Depends on:** MET-002, MET-003
- **Primary paths:** `services/ledger-worker`, `crates/storage`, `services/control-plane-api`
- **Expected outputs:**
  - projection jobs
  - budget threshold evaluation
  - usage summary APIs
- **Acceptance criteria:**
  - projection lag is measurable
  - budget threshold events are emitted once per threshold crossing
  - read models can be repaired without ledger mutation

### MET-005 - Implement billing and usage analytics endpoints

- **Phase:** PI-3
- **Stream:** Metering, Ledger, and Billing
- **Size:** M
- **Depends on:** MET-004, CTL-001
- **Primary paths:** `services/control-plane-api`, `schemas/openapi`
- **Expected outputs:**
  - aggregated usage endpoints
  - balance endpoints
  - billing export endpoints
  - payment record and status endpoints for the initial collection flow
- **Acceptance criteria:**
  - APIs support tenant/project/time filtering
  - responses are paginated for large datasets
  - exports are asynchronous where needed
  - the first payment collection path is modeled for WeChat Pay without leaking provider-specific semantics into ledger records

### SEC-001 - Define RBAC model, scopes, and authorization middleware

- **Phase:** PI-0
- **Stream:** Security, Identity, and Compliance
- **Size:** M
- **Depends on:** FND-001, CTL-001
- **Primary paths:** `crates/authn-authz`, `services/control-plane-api`, `services/gateway-api`
- **Expected outputs:**
  - role model
  - scope evaluation
  - authorization middleware
- **Acceptance criteria:**
  - tenant and platform scopes are distinct
  - forbidden responses are normalized
  - authorization decisions are traceable

### SEC-002 - Implement OIDC/SSO login flow for console users

- **Phase:** PI-3
- **Stream:** Security, Identity, and Compliance
- **Size:** M
- **Depends on:** CTL-004, SEC-001
- **Primary paths:** `apps/console-web`, `services/control-plane-api`, `crates/authn-authz`
- **Expected outputs:**
  - OIDC login flow
  - session handling
  - group-to-role mapping
- **Acceptance criteria:**
  - interactive login works against a reference IdP
  - logout and session expiry are handled cleanly
  - role mapping is configurable

### SEC-003 - Implement secret reference model and provider credential storage boundary

- **Phase:** PI-1
- **Stream:** Security, Identity, and Compliance
- **Size:** M
- **Depends on:** FND-004, DB-001
- **Primary paths:** `crates/storage`, `services/control-plane-api`, `crates/config`
- **Expected outputs:**
  - secret reference abstraction
  - credential vault boundary
  - rotation hooks
- **Acceptance criteria:**
  - plaintext provider secrets are never returned by API
  - secret references can be resolved by authorized services only
  - rotation is auditable

### SEC-004 - Build audit event pipeline and retention controls

- **Phase:** PI-2
- **Stream:** Security, Identity, and Compliance
- **Size:** M
- **Depends on:** DB-002, SEC-001, OBS-001
- **Primary paths:** `services/audit-worker`, `crates/queue`, `crates/storage`
- **Expected outputs:**
  - audit event model
  - durable audit worker
  - retention/archive rules
- **Acceptance criteria:**
  - security-sensitive actions emit audit events
  - audit queries are filterable by actor and resource
  - retention policies are testable

### OBS-001 - Implement telemetry crate and platform-wide OpenTelemetry conventions

- **Phase:** PI-0
- **Stream:** Observability, SRE, and Runtime
- **Size:** M
- **Depends on:** FND-001, FND-003
- **Primary paths:** `crates/telemetry`, `infra/monitoring`, `services/*`
- **Expected outputs:**
  - trace helpers
  - metrics registry
  - structured log conventions
- **Acceptance criteria:**
  - all services emit traces, metrics, and logs with shared resource tags
  - trace IDs propagate across HTTP and events
  - collector config is documented

### OBS-002 - Build service-level dashboards and SLO definitions

- **Phase:** PI-1
- **Stream:** Observability, SRE, and Runtime
- **Size:** M
- **Depends on:** OBS-001, GWT-001, CTL-001
- **Primary paths:** `infra/monitoring`, `docs/runbooks`
- **Expected outputs:**
  - Grafana dashboards
  - latency/error/availability SLOs
  - runbooks
- **Acceptance criteria:**
  - dashboards exist for gateway, control plane, and workers
  - SLO calculations are reproducible
  - runbooks link alerts to operational actions

### OBS-003 - Implement alert routing and incident notification pipeline

- **Phase:** PI-2
- **Stream:** Observability, SRE, and Runtime
- **Size:** S
- **Depends on:** OBS-002, SEC-004
- **Primary paths:** `infra/monitoring`, `services/notification-worker`
- **Expected outputs:**
  - alert rules
  - notification fan-out
  - severity taxonomy
- **Acceptance criteria:**
  - high-severity incidents trigger notifications
  - alert noise is rate-limited
  - test alerts can be fired safely

### OBS-004 - Create load, soak, and failure-injection test suite

- **Phase:** PI-3
- **Stream:** Observability, SRE, and Runtime
- **Size:** L
- **Depends on:** GWT-005, RTE-004, MET-002
- **Primary paths:** `crates/testing-kit`, `infra/scripts`, `docs/runbooks`
- **Expected outputs:**
  - performance test scenarios
  - failure injection harness
  - capacity baseline report
- **Acceptance criteria:**
  - test suite covers concurrency, retry storms, and degraded dependencies
  - results are versioned and comparable
  - capacity thresholds are documented

### QAR-001 - Build protocol contract tests and golden fixtures

- **Phase:** PI-1
- **Stream:** QA, Release, and Documentation
- **Size:** M
- **Depends on:** GWT-004, PAD-002, FND-006
- **Primary paths:** `crates/testing-kit`, `schemas/examples`, `crates/protocol-openai`
- **Expected outputs:**
  - golden fixtures
  - contract runner
  - fixture update workflow
- **Acceptance criteria:**
  - protocol contract tests run in CI
  - fixtures cover success and failure cases
  - breaking fixture deltas require review

### QAR-002 - Create end-to-end integration environment and seeded demo tenant

- **Phase:** PI-2
- **Stream:** QA, Release, and Documentation
- **Size:** M
- **Depends on:** CTL-005, GWT-005, MET-002
- **Primary paths:** `infra/docker`, `infra/scripts`, `apps/console-web`
- **Expected outputs:**
  - seed scripts
  - e2e test environment
  - reference demo scenario
- **Acceptance criteria:**
  - fresh environment can be seeded repeatably
  - smoke tests cover UI + API + gateway flow
  - demo tenant credentials are generated securely

### QAR-003 - Implement release automation, versioning policy, and changelog generation

- **Phase:** PI-2
- **Stream:** QA, Release, and Documentation
- **Size:** M
- **Depends on:** FND-005
- **Primary paths:** `.github/workflows`, `docs/runbooks`, `Cargo.toml`, `package.json`
- **Expected outputs:**
  - release pipeline
  - versioning policy
  - generated changelogs
- **Acceptance criteria:**
  - tagging a release produces versioned artifacts
  - workspace versions are consistent
  - release notes are derived from merged changes

### QAR-004 - Create agent-facing implementation guides and definition-of-done checklists

- **Phase:** PI-0
- **Stream:** QA, Release, and Documentation
- **Size:** S
- **Depends on:** None
- **Primary paths:** `docs/execution`, `README.md`
- **Expected outputs:**
  - execution docs
  - task templates
  - checklists
- **Acceptance criteria:**
  - agents can discover work without reading the whole spec
  - task handoff template exists
  - checklists align with CI and review requirements
