# 24. Data Storage Design

[Back to Docs Index](../README.md)

For coding-facing persistence and messaging constraints, read [Persistence Guidance](persistence-guidance.md) and [Event and Message Contracts](event-and-message-contracts.md) alongside this document.

Preferred rule:

- treat this document as the logical storage and eventing shape, not as a requirement to create every store, topic, or pipeline on day one

### 24.1 PostgreSQL

Primary use cases:

- tenants and projects
- users and permissions
- provider resources metadata
- route policies
- price tables
- immutable ledger entries
- projections
- audit metadata index
- idempotency records

### 24.2 Redis

Primary use cases:

- hot auth cache
- route snapshot cache
- distributed rate limiting
- leader lease or short-lived coordination
- circuit breaker state
- ephemeral realtime session state
- ephemeral MCP session state where transport semantics require it
- transient A2A task-continuation coordination state

Redis must not be a single point of total platform failure. The system should degrade safely if Redis is unavailable.

Guardrail:

- loss of Redis must not erase authoritative task, route, budget, or ledger truth
- stateful protocol continuation may degrade or require reconnection, but the control-plane explanation record must remain reconstructable from durable stores

### 24.3 Vector Store (Semantic Cache)

Primary use cases:

- semantic cache embedding storage and similarity search
- prompt fingerprinting for cache-eligible request detection

Recommended approach:

- start with pgvector (PostgreSQL extension) for initial implementation to minimize infrastructure complexity
- support migration to a dedicated vector store (e.g., Qdrant, Milvus) if scale demands it
- vector store unavailability should result in cache bypass, not request failure

### 24.4 Object Storage

Potential use cases:

- exported reports
- audit archives
- raw event dumps
- large trace payload artifacts

### 24.4 Analytics Store

A columnar analytics system such as ClickHouse may be introduced for:

- route performance analytics
- cost analytics
- historical request diagnostics
- anomaly detection queries


---


## 25. Event-Driven Flows

### 25.1 Event Bus Usage

Use a durable event bus for asynchronous pipelines such as:

- usage ingestion
- audit event fan-out
- notifications
- route score recalculation
- synthetic health result ingestion
- billing export generation

Guardrail:

- event delivery should support retry and fan-out, but should not become the only authoritative home of routing, admission, or billing decisions

### 25.2 Core Event Types

- `request.completed`
- `usage.recorded`
- `ledger.entry.created`
- `route.target.quarantined`
- `provider.resource.degraded`
- `policy.violation.detected`
- `budget.threshold.exceeded`
- `audit.event.created`
- `a2a.task.delegated`
- `a2a.task.completed`
- `a2a.task.failed`
- `semantic.cache.hit`
- `semantic.cache.invalidated`
- `agent.identity.registered`
- `agent.identity.deactivated`

Recommended refinement:

- keep authoritative billing and routing artifacts as durable records first, then emit events from those commit boundaries
- prefer event names that read as completed facts over transport-oriented verbs
- avoid publishing raw prompt, tool payload, or session transcript content into shared event streams by default

### 25.3 Delivery Guarantees

At least-once delivery is acceptable if consumers are idempotent.


---


## 26. Realtime and Streaming

### 26.1 Streaming Requirements

The gateway must support:

- server-sent events
- chunked HTTP streaming
- optional WebSocket-based realtime protocols
- WebRTC session setup with a distinct browser/client connection flow when the upstream protocol expects it

### 26.2 Stream Handling Principles

- preserve ordering of deltas
- minimize buffering
- propagate cancellation quickly
- ensure final usage records are emitted even on stream interruption where possible
- preserve a correlation handle for session-oriented protocols such as realtime call IDs or task IDs

### 26.3 Realtime Gateway

If realtime traffic becomes operationally distinct, it should be split into `services/realtime-gateway` rather than forcing all behavior into the main HTTP service.

Design implication:

- model browser-facing realtime setup and server-side control or monitoring as separate but correlated channels
- avoid assuming one long-lived socket is the only source of truth for a session lifecycle

---
