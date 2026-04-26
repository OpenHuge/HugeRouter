# PI-2 — Multi-Protocol and Reliability

[Back to Task Catalog Index](README.md)

Expansion tasks for reliability, diagnostics, multi-protocol support, and integration maturity.

## Recommended execution order

- **CTL-006** — Implement provider and route management UI with diagnostics surfaces _(depends on: CTL-003, CTL-004, RTE-005)_
- **GWT-006** — Add Anthropic-native northbound protocol support _(depends on: GWT-003, PAD-003, RTE-002)_
- **GWT-007** — Add Gemini-native northbound protocol support _(depends on: GWT-003, PAD-004, RTE-002)_
- **MET-002** — Build ledger worker for immutable event ingestion and deduplication _(depends on: MET-001, DB-001, DB-002)_
- **OBS-003** — Implement alert routing and incident notification pipeline _(depends on: OBS-002, SEC-004)_
- **PAD-003** — Implement Anthropic upstream adapter _(depends on: PAD-001, GWT-003)_
- **PAD-004** — Implement Gemini upstream adapter _(depends on: PAD-001, GWT-003)_
- **QAR-002** — Create end-to-end integration environment and seeded demo tenant _(depends on: CTL-005, GWT-005, MET-002)_
- **QAR-003** — Implement release automation, versioning policy, and changelog generation _(depends on: FND-005)_
- **RTE-003** — Implement active health probes and passive health aggregation _(depends on: RTE-001, DB-002, OBS-001)_
- **RTE-004** — Implement retry budgets, circuit breaking, and quarantine logic _(depends on: RTE-002, RTE-003, OBS-001)_
- **RTE-005** — Expose route diagnostics APIs and explanation model _(depends on: RTE-002, RTE-003, CTL-001)_
- **SEC-004** — Build audit event pipeline and retention controls _(depends on: DB-002, SEC-001, OBS-001)_

## Detailed tasks

### CTL-006 — Implement provider and route management UI with diagnostics surfaces

**Stream:** Control Plane and Console  
**Owners:** frontend-console  
**Depends on:** CTL-003, CTL-004, RTE-005

**Paths:**

- `apps/console-web/src/routes/providers`
- `apps/console-web/src/routes/routes`

**Outputs:**

- provider detail pages
- route editors
- route diagnostics panels
- relay-aware provider resource forms and validation flows
- compatibility-profile selection and preset validation for common relay families

**Acceptance criteria:**

- operators can inspect route health and policy resolution
- form edits are optimistic only where safe
- dangerous operations require confirmation
- operators can create and manage third-party relay resources by endpoint and key across multiple compatibility families, including One API-like, New API-like, Sub2API-like, LiteLLM-like, LMRouter-like, and generic OpenAI-compatible gateways

### GWT-006 — Add Anthropic-native northbound protocol support

**Stream:** Gateway and Protocol Ingress  
**Owners:** gateway  
**Depends on:** GWT-003, PAD-003, RTE-002

**Paths:**

- `crates/protocol-anthropic`
- `services/gateway-api`

**Outputs:**

- Anthropic request/response mapping
- protocol-specific validation

**Acceptance criteria:**

- Anthropic-native message requests reach supported upstreams
- provider-specific fields are preserved when possible
- contract tests pass

### GWT-007 — Add Gemini-native northbound protocol support

**Stream:** Gateway and Protocol Ingress  
**Owners:** gateway  
**Depends on:** GWT-003, PAD-004, RTE-002

**Paths:**

- `crates/protocol-gemini`
- `services/gateway-api`

**Outputs:**

- Gemini request/response mapping
- tool/function compatibility notes

**Acceptance criteria:**

- Gemini text generation works end-to-end
- unsupported constructs fail with explicit compatibility errors
- fixtures cover mapping edge cases

### MET-002 — Build ledger worker for immutable event ingestion and deduplication

**Stream:** Metering, Ledger, and Billing  
**Owners:** billing-platform  
**Depends on:** MET-001, DB-001, DB-002

**Paths:**

- `services/ledger-worker`
- `crates/ledger-models`
- `crates/storage`

**Outputs:**

- immutable ledger persistence
- deduplication logic
- repair/replay hooks

**Acceptance criteria:**

- duplicate usage events are ignored safely
- ledger writes are append-only
- replay against a snapshot produces deterministic results

### OBS-003 — Implement alert routing and incident notification pipeline

**Stream:** Observability, SRE, and Runtime  
**Owners:** sre  
**Depends on:** OBS-002, SEC-004

**Paths:**

- `infra/monitoring`
- `services/notification-worker`

**Outputs:**

- alert rules
- notification fan-out
- severity taxonomy

**Acceptance criteria:**

- high-severity incidents trigger notifications
- alert noise is rate-limited
- test alerts can be fired safely

### PAD-003 — Implement Anthropic upstream adapter

**Stream:** Provider Adapters  
**Owners:** provider-anthropic  
**Depends on:** PAD-001, GWT-003

**Paths:**

- `crates/provider-anthropic`

**Outputs:**

- Anthropic provider client
- message API integration
- usage extraction hooks

**Acceptance criteria:**

- adapter passes conformance fixtures
- unsupported native features are surfaced clearly
- timeout and region configs are supported

### PAD-004 — Implement Gemini upstream adapter

**Stream:** Provider Adapters  
**Owners:** provider-gemini  
**Depends on:** PAD-001, GWT-003

**Paths:**

- `crates/provider-gemini`

**Outputs:**

- Gemini provider client
- compatibility layer
- usage extraction hooks

**Acceptance criteria:**

- adapter passes conformance fixtures
- model capability metadata is discoverable
- error normalization is documented

### QAR-002 — Create end-to-end integration environment and seeded demo tenant

**Stream:** QA, Release, and Documentation  
**Owners:** qa, platform  
**Depends on:** CTL-005, GWT-005, MET-002

**Paths:**

- `infra/docker`
- `infra/scripts`
- `apps/console-web`

**Outputs:**

- seed scripts
- e2e test environment
- reference demo scenario

**Acceptance criteria:**

- fresh environment can be seeded repeatably
- smoke tests cover UI + API + gateway flow
- demo tenant credentials are generated securely

### QAR-003 — Implement release automation, versioning policy, and changelog generation

**Stream:** QA, Release, and Documentation  
**Owners:** platform  
**Depends on:** FND-005

**Paths:**

- `.github/workflows`
- `docs/runbooks`
- `Cargo.toml`
- `package.json`

**Outputs:**

- release pipeline
- versioning policy
- generated changelogs

**Acceptance criteria:**

- tagging a release produces versioned artifacts
- workspace versions are consistent
- release notes are derived from merged changes

### RTE-003 — Implement active health probes and passive health aggregation

**Stream:** Routing and Reliability  
**Owners:** routing, sre  
**Depends on:** RTE-001, DB-002, OBS-001

**Paths:**

- `services/edge-probe`
- `services/routing-worker`
- `crates/routing-engine`

**Outputs:**

- probe scheduler
- health score aggregation
- route health persistence

**Acceptance criteria:**

- probe results update route health state
- cheap and billable probe modes are separated
- quarantine thresholds are configurable

### RTE-004 — Implement retry budgets, circuit breaking, and quarantine logic

**Stream:** Routing and Reliability  
**Owners:** routing, sre  
**Depends on:** RTE-002, RTE-003, OBS-001

**Paths:**

- `crates/routing-engine`
- `services/gateway-api`
- `services/routing-worker`

**Outputs:**

- retry budget policy
- circuit breaker state
- quarantine transitions

**Acceptance criteria:**

- retry loops are bounded and observable
- bad routes can be quarantined automatically
- manual override APIs exist

### RTE-005 — Expose route diagnostics APIs and explanation model

**Stream:** Routing and Reliability  
**Owners:** routing, control-plane  
**Depends on:** RTE-002, RTE-003, CTL-001

**Paths:**

- `services/control-plane-api`
- `crates/routing-engine`
- `schemas/openapi`

**Outputs:**

- diagnostics endpoints
- route explanation model
- recent failure summaries

**Acceptance criteria:**

- operators can query why a route was or was not selected
- health and score inputs are visible with redaction
- OpenAPI docs include example responses

### SEC-004 — Build audit event pipeline and retention controls

**Stream:** Security, Identity, and Compliance  
**Owners:** security, data-platform  
**Depends on:** DB-002, SEC-001, OBS-001

**Paths:**

- `services/audit-worker`
- `crates/queue`
- `crates/storage`

**Outputs:**

- audit event model
- durable audit worker
- retention/archive rules

**Acceptance criteria:**

- security-sensitive actions emit audit events
- audit queries are filterable by actor and resource
- retention policies are testable
