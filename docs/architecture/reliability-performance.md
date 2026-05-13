# 18. Reliability and Performance

[Back to Docs Index](../README.md)

### 18.1 Availability Targets

Suggested targets:

- control plane API: 99.9%
- gateway API: 99.95%
- tenant billing projections: eventual consistency within acceptable delay

### 18.2 Latency Targets

Targets will depend on provider latency, but gateway-added overhead should remain bounded.

Non-streaming gateway overhead target:
- p50 under 15 ms
- p95 under 40 ms in healthy conditions

Streaming overhead target:
- first-byte relay overhead under 20 ms beyond upstream readiness

### 18.3 Resilience Patterns

- circuit breakers
- bounded retries
- bulkheads
- queue-based asynchronous projections
- local cache snapshots for auth and routing hot paths
- degraded-mode behavior when Redis or analytics systems are slow

### 18.4 Backpressure

The system must enforce backpressure for:

- too many concurrent streams
- exhausted project budget
- provider-side throttling
- global traffic surge

### 18.5 Idempotency

Support idempotency keys for retriable request classes where safe.

---
