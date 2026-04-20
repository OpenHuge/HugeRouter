# 33. Service-by-Service Detailed Responsibilities

[Back to Docs Index](../README.md)

This document describes preferred responsibility boundaries, not a requirement to create every listed service or crate in the first implementation.

Preferred rule:

- start with fewer deployables and fewer crates when ownership is still clear
- split a service or crate only when lifecycle, scaling, isolation, or review boundaries become materially different
- avoid empty plugin, registry, or runtime-composition layers unless a second concrete caller already forces the abstraction

### 33.1 `services/gateway-api`

**Purpose:** Main northbound ingress for synchronous HTTP request traffic.

Responsibilities:

- authenticate requests
- parse protocol-specific payloads
- build IR
- evaluate policy
- select route
- resolve provider adapter from registry
- execute the composed adapter pipeline
- relay streams
- emit telemetry
- publish usage events

Must not:

- perform heavy analytics queries
- directly compute complex billing projections on hot path
- directly own admin workflows
- hard-wire provider-specific behavior into route handlers

### 33.2 `services/control-plane-api`

**Purpose:** CRUD and management API for platform resources.

Responsibilities:

- manage tenants, projects, users, credentials
- manage providers and route policies
- manage budgets and pricing tables
- expose usage summaries and diagnostics metadata
- enforce admin/tenant role scopes
- expose discoverable manifests and capability metadata for pluggable modules

### 33.3 `services/ledger-worker`

**Purpose:** Persist immutable usage and billing records and maintain projections.

Responsibilities:

- consume usage events
- normalize and deduplicate
- create ledger entries
- update projection tables
- emit threshold events

### 33.4 `services/routing-worker`

**Purpose:** Background route intelligence service.

Responsibilities:

- aggregate route quality stats
- recalculate health scores
- maintain route recommendation data
- manage quarantine transitions
- precompute capability or manifest compatibility hints where useful

### 33.5 `services/audit-worker`

**Purpose:** Durable audit processing.

Responsibilities:

- persist audit events
- enrich events with actor metadata
- export audit streams if needed
- apply retention or archive rules

### 33.6 `services/notification-worker`

**Purpose:** Notification fan-out.

Responsibilities:

- email budget alerts
- webhook dispatch
- incident signals
- admin notifications

### 33.7 `services/realtime-gateway`

**Purpose:** Specialized handling for bidirectional or long-lived realtime sessions.

### 33.8 `services/edge-probe`

**Purpose:** Run active probes against provider resources or regional edges.

Each service should have an explicit **composition root** responsible for wiring registries, middleware, policies, caches, and external clients. Business logic crates should not self-bootstrap hidden globals.

---

## 34. Package-by-Package Detailed Responsibilities

### 34.1 `crates/core-domain`

Contains:

- strongly typed IDs
- shared enums
- common domain errors
- value objects
- base traits

### 34.2 `crates/protocol-ir`

Contains:

- request and response IR
- capability definitions
- semantic validation rules
- extension envelopes for protocol- or provider-specific metadata

### 34.3 Protocol crates

`protocol-openai`, `protocol-anthropic`, `protocol-gemini`, `protocol-realtime`

Contain:

- protocol request/response models
- parsing and serialization
- protocol-specific validation
- compatibility layers

### 34.4 `crates/provider-traits`

Contains:

- adapter interfaces
- execution context
- common adapter result types
- error normalization contracts

### 34.5 `crates/plugin-sdk`

Contains:

- stable extension contracts
- manifest types
- capability descriptors
- config schema contracts
- versioning rules for pluggable modules

### 34.6 `crates/plugin-registry`

Contains:

- module registration APIs
- runtime lookup and manifest indexing
- duplicate/conflict validation
- discovery interfaces for control plane and diagnostics

### 34.7 `crates/runtime-composition`

Contains:

- composition root helpers
- middleware chain assembly
- request pipeline composition
- shared bootstrap conventions for services

### 34.8 `crates/routing-engine`

Contains:

- route candidate model
- scoring functions
- route strategies
- fallback planner
- route simulation logic

### 34.9 `crates/policy-engine`

Contains:

- request policies
- route policies
- response policies
- policy evaluation output types

### 34.10 `crates/authn-authz`

Contains:

- key validation
- JWT validation
- tenant scope evaluation
- RBAC helpers
- auth middleware

### 34.11 `crates/metering`

Contains:

- usage extraction contracts
- normalization logic
- unit calculators
- pluggable metering sink interfaces where needed

### 34.12 `crates/ledger-models`

Contains:

- usage event schema
- ledger entry schema
- balance projection models

### 34.13 `crates/storage`

Contains:

- repositories
- SQL query modules
- transaction helpers
- migration glue

### 34.14 `crates/cache`

Contains:

- Redis client wrappers
- local fallback caches
- circuit breaker state helpers

### 34.15 `crates/telemetry`

Contains:

- tracing middleware
- log enrichers
- metrics wrappers
- trace propagation helpers

### 34.16 `crates/testing-kit`

Contains:

- fixture builders
- fake adapters
- integration test harnesses
- protocol conformance helpers
- registry and composition test helpers

### 34.17 Architectural Dependency Rule

Dependencies should flow in this direction:

1. domain and IR crates
2. contracts and SDK crates
3. feature engines such as routing, policy, metering
4. adapters and infrastructure implementations
5. service composition roots

Service crates may depend on all lower layers. Lower layers must not depend on service crates.

---
