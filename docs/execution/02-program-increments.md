# Program Increments

[Back to Execution Index](README.md)

## PI-0 - Foundation

**Goal:** Make the monorepo buildable, testable, observable, and ready for multiple agents.

| Task ID | Task | Workstream | Depends On |
|---|---|---|---|
| CTL-001 | Create control plane API service skeleton with typed CRUD conventions | Control Plane and Console | FND-001, FND-004 |
| CTL-004 | Build TanStack Start + Mantine application shell and authenticated app layout | Control Plane and Console | FND-002, FND-006 |
| DB-001 | Design relational schema, migrations, and repository primitives | Data Storage and Events | FND-001, FND-003 |
| DB-002 | Implement event bus abstractions and NATS-backed publishers/consumers | Data Storage and Events | FND-001, FND-003 |
| FND-001 | Initialize Rust workspace and service skeletons | Foundation and Monorepo | None |
| FND-002 | Initialize pnpm + Turbo workspace and TanStack Start application shell | Foundation and Monorepo | None |
| FND-003 | Provision local development stack and containerized runtime | Foundation and Monorepo | None |
| FND-004 | Establish shared configuration, secrets, and environment loading | Foundation and Monorepo | FND-001 |
| FND-005 | Set up CI pipelines, quality gates, and caching strategy | Foundation and Monorepo | FND-001, FND-002 |
| FND-006 | Create schema pipeline for OpenAPI, JSON Schema, and generated TS client | Foundation and Monorepo | FND-002 |
| OBS-001 | Implement telemetry crate and platform-wide OpenTelemetry conventions | Observability, SRE, and Runtime | FND-001, FND-003 |
| QAR-004 | Create agent-facing implementation guides and definition-of-done checklists | QA, Release, and Documentation | None |
| SEC-001 | Define RBAC model, scopes, and authorization middleware | Security, Identity, and Compliance | FND-001, CTL-001 |

**Exit criteria**

- Rust and pnpm workspaces compile.
- Local Postgres, Redis, NATS, and OTel stack can be started with one command.
- Control plane skeleton and TanStack + Mantine app shell exist.
- RBAC, telemetry conventions, schema generation, and CI gates are in place.

## PI-1 - Core Platform MVP

**Goal:** Deliver a usable management plane and a first northbound gateway path.

| Task ID | Task | Workstream | Depends On |
|---|---|---|---|
| CTL-002 | Implement tenant, project, environment, and API key CRUD | Control Plane and Console | CTL-001, DB-001, SEC-001 |
| CTL-003 | Implement provider, credential, route policy, and route set management APIs | Control Plane and Console | CTL-001, PAD-001, RTE-001, SEC-003 |
| CTL-005 | Implement tenant/project management UI flows | Control Plane and Console | CTL-002, CTL-004 |
| GWT-001 | Implement gateway HTTP server skeleton and middleware chain | Gateway and Protocol Ingress | FND-001, FND-003, FND-004 |
| GWT-002 | Implement API key authentication and tenant/project resolution | Gateway and Protocol Ingress | CTL-001, SEC-001 |
| GWT-003 | Define protocol IR v1 with request, response, and capability models | Gateway and Protocol Ingress | FND-001 |
| GWT-004 | Ship OpenAI-compatible northbound chat/completions ingress | Gateway and Protocol Ingress | GWT-001, GWT-003, PAD-001, PAD-002, RTE-002, MET-001 |
| GWT-005 | Implement SSE streaming relay and backpressure-safe response path | Gateway and Protocol Ingress | GWT-004 |
| MET-001 | Define usage event schema and hot-path usage extraction hooks | Metering, Ledger, and Billing | GWT-003, DB-002 |
| OBS-002 | Build service-level dashboards and SLO definitions | Observability, SRE, and Runtime | OBS-001, GWT-001, CTL-001 |
| PAD-001 | Define provider adapter trait model and adapter conformance test kit | Provider Adapters | GWT-003, FND-001 |
| PAD-002 | Implement OpenAI upstream adapter | Provider Adapters | PAD-001, GWT-003 |
| QAR-001 | Build protocol contract tests and golden fixtures | QA, Release, and Documentation | GWT-004, PAD-002, FND-006 |
| RTE-001 | Implement route, provider target, and policy domain models | Routing and Reliability | GWT-003, DB-001 |
| RTE-002 | Implement routing engine MVP for static selection and fallback | Routing and Reliability | RTE-001, PAD-001 |
| SEC-003 | Implement secret reference model and provider credential storage boundary | Security, Identity, and Compliance | FND-004, DB-001 |

**Exit criteria**

- Operators can manage tenants, API keys, providers, and routes.
- Clients can send OpenAI-compatible requests through the gateway.
- At least one upstream adapter works with streaming and non-streaming flows.
- Static route selection and usage event emission exist.

## PI-2 - Multi-Protocol and Reliability

**Goal:** Expand protocol coverage and make routing explainable and resilient.

| Task ID | Task | Workstream | Depends On |
|---|---|---|---|
| CTL-006 | Implement provider and route management UI with diagnostics surfaces | Control Plane and Console | CTL-003, CTL-004, RTE-005 |
| GWT-006 | Add Anthropic-native northbound protocol support | Gateway and Protocol Ingress | GWT-003, PAD-003, RTE-002 |
| GWT-007 | Add Gemini-native northbound protocol support | Gateway and Protocol Ingress | GWT-003, PAD-004, RTE-002 |
| MET-002 | Build ledger worker for immutable event ingestion and deduplication | Metering, Ledger, and Billing | MET-001, DB-001, DB-002 |
| OBS-003 | Implement alert routing and incident notification pipeline | Observability, SRE, and Runtime | OBS-002, SEC-004 |
| PAD-003 | Implement Anthropic upstream adapter | Provider Adapters | PAD-001, GWT-003 |
| PAD-004 | Implement Gemini upstream adapter | Provider Adapters | PAD-001, GWT-003 |
| QAR-002 | Create end-to-end integration environment and seeded demo tenant | QA, Release, and Documentation | CTL-005, GWT-005, MET-002 |
| QAR-003 | Implement release automation, versioning policy, and changelog generation | QA, Release, and Documentation | FND-005 |
| RTE-003 | Implement active health probes and passive health aggregation | Routing and Reliability | RTE-001, DB-002, OBS-001 |
| RTE-004 | Implement retry budgets, circuit breaking, and quarantine logic | Routing and Reliability | RTE-002, RTE-003, OBS-001 |
| RTE-005 | Expose route diagnostics APIs and explanation model | Routing and Reliability | RTE-002, RTE-003, CTL-001 |
| SEC-004 | Build audit event pipeline and retention controls | Security, Identity, and Compliance | DB-002, SEC-001, OBS-001 |

**Exit criteria**

- Anthropic and Gemini are supported in at least one northbound or upstream path.
- Health probes, retry budgets, quarantine logic, and route diagnostics are implemented.
- Audit pipeline and incident alerts exist.
- A seeded end-to-end environment is available.

## PI-3 - Billing and Enterprise Hardening

**Goal:** Move from usage capture to accounting-grade ledger and operational maturity.

| Task ID | Task | Workstream | Depends On |
|---|---|---|---|
| CTL-007 | Build usage, budget, and billing dashboards | Control Plane and Console | MET-004, MET-005, CTL-004 |
| MET-003 | Implement pricing engine with token, cached-token, and multimodal units | Metering, Ledger, and Billing | MET-002, CTL-003 |
| MET-004 | Build balance, budget, and projection read models | Metering, Ledger, and Billing | MET-002, MET-003 |
| MET-005 | Implement billing and usage analytics endpoints | Metering, Ledger, and Billing | MET-004, CTL-001 |
| OBS-004 | Create load, soak, and failure-injection test suite | Observability, SRE, and Runtime | GWT-005, RTE-004, MET-002 |
| SEC-002 | Implement OIDC/SSO login flow for console users | Security, Identity, and Compliance | CTL-004, SEC-001 |

**Exit criteria**

- Immutable ledger and pricing engine are live.
- Budget and balance projections are queryable.
- Usage and billing dashboards exist in the console.
- SSO and load/performance baselines are in place.

## PI-4 - Ecosystem and Realtime

**Goal:** Add advanced deployment options, gateway federation, and realtime support.

| Task ID | Task | Workstream | Depends On |
|---|---|---|---|
| GWT-008 | Build realtime session gateway skeleton | Gateway and Protocol Ingress | FND-001, OBS-001, RTE-004 |
| PAD-005 | Implement gateway-of-gateways adapter for upstream transit systems | Provider Adapters | PAD-001, RTE-004 |

**Exit criteria**

- Realtime gateway skeleton is operational.
- Gateway-of-gateways adapter is supported.
- Advanced policy and ecosystem integrations can be layered on top.
