# PI-1 — Core Platform MVP

[Back to Task Catalog Index](README.md)

Core platform MVP tasks that produce the first working control plane and gateway flow.

## Recommended execution order

- **CTL-002** — Implement tenant, project, environment, and API key CRUD _(depends on: CTL-001, DB-001, SEC-001)_
- **CTL-003** — Implement provider, credential, route policy, and route set management APIs _(depends on: CTL-001, PAD-001, RTE-001, SEC-003)_
- **CTL-005** — Implement tenant/project management UI flows _(depends on: CTL-002, CTL-004)_
- **GWT-001** — Implement gateway HTTP server skeleton and middleware chain _(depends on: FND-001, FND-003, FND-004)_
- **GWT-002** — Implement API key authentication and tenant/project resolution _(depends on: CTL-001, SEC-001)_
- **GWT-003** — Define protocol IR v1 with request, response, and capability models _(depends on: FND-001)_
- **GWT-004** — Ship OpenAI-compatible northbound chat/completions ingress _(depends on: GWT-001, GWT-003, PAD-001, PAD-002, RTE-002, MET-001)_
- **GWT-005** — Implement SSE streaming relay and backpressure-safe response path _(depends on: GWT-004)_
- **MET-001** — Define usage event schema and hot-path usage extraction hooks _(depends on: GWT-003, DB-002)_
- **OBS-002** — Build service-level dashboards and SLO definitions _(depends on: OBS-001, GWT-001, CTL-001)_
- **PAD-001** — Define provider adapter trait model and adapter conformance test kit _(depends on: GWT-003, FND-001)_
- **PAD-002** — Implement OpenAI upstream adapter _(depends on: PAD-001, GWT-003)_
- **QAR-001** — Build protocol contract tests and golden fixtures _(depends on: GWT-004, PAD-002, FND-006)_
- **RTE-001** — Implement route, provider target, and policy domain models _(depends on: GWT-003, DB-001)_
- **RTE-002** — Implement routing engine MVP for static selection and fallback _(depends on: RTE-001, PAD-001)_
- **SEC-003** — Implement secret reference model and provider credential storage boundary _(depends on: FND-004, DB-001)_

## Detailed tasks

### CTL-002 — Implement tenant, project, environment, and API key CRUD

**Stream:** Control Plane and Console  
**Owners:** control-plane  
**Depends on:** CTL-001, DB-001, SEC-001

**Paths:**

- `services/control-plane-api`
- `crates/storage`
- `crates/core-domain`

**Outputs:**

- resource CRUD endpoints
- API key issuance/rotation
- resource validation

**Acceptance criteria:**

- CRUD endpoints are idempotent where expected
- API key rotation invalidates old secrets
- OpenAPI and TS client are updated

### CTL-003 — Implement provider, credential, route policy, and route set management APIs

**Stream:** Control Plane and Console  
**Owners:** control-plane  
**Depends on:** CTL-001, PAD-001, RTE-001, SEC-003

**Paths:**

- `services/control-plane-api`
- `crates/storage`
- `crates/provider-traits`
- `crates/routing-engine`

**Outputs:**

- provider CRUD
- credential binding
- route policy CRUD

**Acceptance criteria:**

- route policies can be created and validated from API
- secret references are stored without plaintext leakage
- policy schema validation errors are actionable

### CTL-005 — Implement tenant/project management UI flows

**Stream:** Control Plane and Console  
**Owners:** frontend-console  
**Depends on:** CTL-002, CTL-004

**Paths:**

- `apps/console-web/src/routes`
- `packages/ts-api-client`

**Outputs:**

- list/detail/create flows
- API key issuance views
- error/loading states

**Acceptance criteria:**

- CRUD flows are fully typed from generated client
- forms validate client-side and server-side
- role-based page guards work

### GWT-001 — Implement gateway HTTP server skeleton and middleware chain

**Stream:** Gateway and Protocol Ingress  
**Owners:** gateway  
**Depends on:** FND-001, FND-003, FND-004

**Paths:**

- `services/gateway-api`
- `crates/sdk-server`
- `crates/telemetry`

**Outputs:**

- HTTP server bootstrap
- middleware pipeline
- health/readiness endpoints

**Acceptance criteria:**

- service starts with trace and metrics middleware
- health endpoints are covered by tests
- configuration is hot-reload safe or explicitly immutable

### GWT-002 — Implement API key authentication and tenant/project resolution

**Stream:** Gateway and Protocol Ingress  
**Owners:** gateway, security  
**Depends on:** CTL-001, SEC-001

**Paths:**

- `services/gateway-api`
- `crates/authn-authz`
- `crates/storage`

**Outputs:**

- API key auth middleware
- tenant/project context injection
- error model

**Acceptance criteria:**

- valid keys resolve tenant and project scopes
- invalid/disabled keys fail with normalized errors
- auth path emits audit and trace metadata

### GWT-003 — Define protocol IR v1 with request, response, and capability models

**Stream:** Gateway and Protocol Ingress  
**Owners:** gateway-architecture  
**Depends on:** FND-001

**Paths:**

- `crates/protocol-ir`
- `crates/core-domain`
- `docs/architecture/protocol-ir-and-protocols.md`

**Outputs:**

- IR data structures
- semantic validation
- provider extension fields

**Acceptance criteria:**

- IR covers text generation MVP
- unknown provider-specific metadata can be preserved
- IR versioning strategy is documented

### GWT-004 — Ship OpenAI-compatible northbound chat/completions ingress

**Stream:** Gateway and Protocol Ingress  
**Owners:** gateway  
**Depends on:** GWT-001, GWT-003, PAD-001, PAD-002, RTE-002, MET-001

**Paths:**

- `services/gateway-api`
- `crates/protocol-openai`
- `schemas/examples`

**Outputs:**

- OpenAI-compatible endpoint
- request parsing to IR
- response mapping from IR/provider result

**Acceptance criteria:**

- non-streaming chat requests work end-to-end
- OpenAI-compatible errors are returned
- golden fixture tests pass

### GWT-005 — Implement SSE streaming relay and backpressure-safe response path

**Stream:** Gateway and Protocol Ingress  
**Owners:** gateway  
**Depends on:** GWT-004

**Paths:**

- `services/gateway-api`
- `crates/sdk-server`
- `crates/protocol-openai`

**Outputs:**

- stream relay
- disconnect handling
- partial usage capture hooks

**Acceptance criteria:**

- streaming responses are flushed incrementally
- client disconnects cancel upstream requests
- stream tests cover partial and terminal events

### MET-001 — Define usage event schema and hot-path usage extraction hooks

**Stream:** Metering, Ledger, and Billing  
**Owners:** billing-platform  
**Depends on:** GWT-003, DB-002

**Paths:**

- `crates/metering`
- `crates/ledger-models`
- `services/gateway-api`

**Outputs:**

- usage event schema
- gateway emission hooks
- idempotency key format

**Acceptance criteria:**

- usage events can be emitted for success and failure paths
- schema version is explicit
- events include tenant/project/route correlation IDs

### OBS-002 — Build service-level dashboards and SLO definitions

**Stream:** Observability, SRE, and Runtime  
**Owners:** sre  
**Depends on:** OBS-001, GWT-001, CTL-001

**Paths:**

- `infra/monitoring`
- `docs/runbooks`

**Outputs:**

- Grafana dashboards
- latency/error/availability SLOs
- runbooks

**Acceptance criteria:**

- dashboards exist for gateway, control plane, and workers
- SLO calculations are reproducible
- runbooks link alerts to operational actions

### PAD-001 — Define provider adapter trait model and adapter conformance test kit

**Stream:** Provider Adapters  
**Owners:** gateway-architecture  
**Depends on:** GWT-003, FND-001

**Paths:**

- `crates/provider-traits`
- `crates/testing-kit`
- `docs/architecture/provider-adapter-system.md`

**Outputs:**

- adapter trait set
- error normalization model
- conformance fixtures

**Acceptance criteria:**

- all adapters implement shared trait surfaces
- adapter tests can run against mocks
- error categories map to routing decisions

### PAD-002 — Implement OpenAI upstream adapter

**Stream:** Provider Adapters  
**Owners:** provider-openai  
**Depends on:** PAD-001, GWT-003

**Paths:**

- `crates/provider-openai`

**Outputs:**

- OpenAI provider client
- streaming and non-streaming calls
- usage extraction hooks

**Acceptance criteria:**

- adapter passes conformance fixtures
- timeouts and retries are configurable
- provider errors map to normalized categories

### QAR-001 — Build protocol contract tests and golden fixtures

**Stream:** QA, Release, and Documentation  
**Owners:** qa, gateway  
**Depends on:** GWT-004, PAD-002, FND-006

**Paths:**

- `crates/testing-kit`
- `schemas/examples`
- `crates/protocol-openai`

**Outputs:**

- golden fixtures
- contract runner
- fixture update workflow

**Acceptance criteria:**

- protocol contract tests run in CI
- fixtures cover success and failure cases
- breaking fixture deltas require review

### RTE-001 — Implement route, provider target, and policy domain models

**Stream:** Routing and Reliability  
**Owners:** routing  
**Depends on:** GWT-003, DB-001

**Paths:**

- `crates/routing-engine`
- `crates/core-domain`
- `services/control-plane-api`

**Outputs:**

- route entities
- policy model
- score input structures

**Acceptance criteria:**

- route entities support weighted and ordered strategies
- validation prevents impossible configurations
- models are reusable by API and workers

### RTE-002 — Implement routing engine MVP for static selection and fallback

**Stream:** Routing and Reliability  
**Owners:** routing  
**Depends on:** RTE-001, PAD-001

**Paths:**

- `crates/routing-engine`
- `services/gateway-api`

**Outputs:**

- route evaluator
- fallback chain execution
- selection reasoning output

**Acceptance criteria:**

- engine selects eligible route deterministically
- fallback occurs on retryable error classes
- selection reason is emitted in traces

### SEC-003 — Implement secret reference model and provider credential storage boundary

**Stream:** Security, Identity, and Compliance  
**Owners:** security, control-plane  
**Depends on:** FND-004, DB-001

**Paths:**

- `crates/storage`
- `services/control-plane-api`
- `crates/config`

**Outputs:**

- secret reference abstraction
- credential vault boundary
- rotation hooks

**Acceptance criteria:**

- plaintext provider secrets are never returned by API
- secret references can be resolved by authorized services only
- rotation is auditable
