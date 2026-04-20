# 24. Data Storage Design

[Back to Docs Index](../README.md)

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

Redis must not be a single point of total platform failure. The system should degrade safely if Redis is unavailable.

### 24.3 Object Storage

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

### 25.2 Core Event Types

- `request.completed`
- `usage.recorded`
- `ledger.entry.created`
- `route.target.quarantined`
- `provider.resource.degraded`
- `policy.violation.detected`
- `budget.threshold.exceeded`
- `audit.event.created`

### 25.3 Delivery Guarantees

At least-once delivery is acceptable if consumers are idempotent.


---


## 26. Realtime and Streaming

### 26.1 Streaming Requirements

The gateway must support:

- server-sent events
- chunked HTTP streaming
- optional WebSocket-based realtime protocols

### 26.2 Stream Handling Principles

- preserve ordering of deltas
- minimize buffering
- propagate cancellation quickly
- ensure final usage records are emitted even on stream interruption where possible

### 26.3 Realtime Gateway

If realtime traffic becomes operationally distinct, it should be split into `services/realtime-gateway` rather than forcing all behavior into the main HTTP service.

---
