# 17. Observability

[Back to Docs Index](../README.md)

### 17.1 Requirements

The observability stack must answer:

- which route was chosen and why
- what upstream was actually called
- where did latency accumulate
- why did a retry occur
- how much usage was billed
- did a policy alter the request or response
- did a cache or circuit breaker trigger

### 17.2 Telemetry Types

- traces
- metrics
- structured logs
- audit events
- route evaluation records
- route receipts
- synthetic probe results

### 17.3 Trace Model

Each request should include:

- gateway request span
- auth span
- policy evaluation span
- route scoring span
- adapter execution span
- stream relay span
- metering span
- ledger emit span

For stateful protocols, the trace model should also preserve stable session and task correlation:

- realtime session or call span
- MCP request or tool-execution span
- A2A task span and task-update spans
- delegation-chain attributes when a task crosses agent boundaries

### 17.4 Key Metrics

- request count
- success rate
- latency percentiles
- stream duration
- first-token latency
- provider error rate
- retry rate
- route fallback rate
- admission-control rejection rate
- quota reservation rejection rate
- token volume
- estimated cost
- gross margin
- auth failure count
- policy denial count
- trust-class traffic mix
- redaction coverage rate
- semantic cache hit rate and similarity distribution
- A2A task delegation count and completion rate
- agent identity traffic mix
- agentic session duration and tool call depth

### 17.5 Log Requirements

Logs should be structured JSON and include:

- trace ID
- request ID
- tenant ID
- project ID
- credential prefix
- protocol family
- requested model alias
- resolved provider target
- target provenance class
- error code
- retry attempt
- route policy ID
- config snapshot ID
- quota reservation ID or outcome
- redaction tier
- caller identity type
- delegated subject or origin principal when present
- realtime call or session ID when applicable
- A2A task ID and task terminal state when applicable

Sensitive payload logging must be disabled by default and opt-in with redaction controls.

### 17.6 Semantic Convention Registry

Observability fields should not be invented ad hoc by each crate or service.

The platform should maintain a shared telemetry semantic registry covering:

- span names
- metric names and units
- event names
- resource attributes
- stable attribute keys
- incubating or experimental attribute keys

Recommended governance model:

- `crates/telemetry` exports typed constants and helpers
- stable fields are documented and versioned
- incubating fields use a clearly marked namespace until proven
- feature teams add new fields through review instead of silently emitting one-off dimensions

### 17.7 AI-Gateway-Specific Semantic Fields

In addition to baseline HTTP and service telemetry, the platform should standardize fields for:

- protocol family
- northbound endpoint family
- requested model alias
- resolved provider target
- route policy ID
- fallback count
- retry classification
- cache decision (semantic cache hit, miss, bypass, or invalidation)
- usage unit family
- estimated and final billable cost
- agent identity type (human, service, agent)
- A2A task ID and delegation chain depth
- agentic session ID and tool call sequence

This follows the OpenTelemetry discipline of semantic conventions while allowing product-specific extensions where the standard has no native concept.

### 17.8 Diagnostics Event Model

Every request should be able to emit a compact diagnostic event timeline containing:

- auth result
- policy result
- selected route candidate set
- excluded candidates and exclusion reasons
- chosen provider target
- quota and budget admission outcome
- retry and fallback transitions
- final normalization and usage extraction result
- protocol-specific lifecycle edges such as realtime session creation, MCP tool dispatch, or A2A task continuation

These events should be structured enough for support tooling and post-incident analysis, not just for log search.

### 17.9 Redaction-First Observability

Most gateway observability failures are data-governance failures masquerading as debugging features.

The platform should therefore implement:

- raw prompt and tool payload capture disabled by default
- redacted structured summaries for support and analytics views
- separate retention tiers for metadata, usage, and sealed payloads
- explicit access controls and audit trails for any sealed payload retrieval
- payload fingerprinting or hashing for correlation without content retention where possible

### 17.10 Replay and Support Capsules

Support and incident tooling should be able to reconstruct failures without requiring unbounded log retention.

The system should produce a replay capsule or support bundle containing:

- route receipt
- config snapshot IDs
- normalized request metadata
- redacted policy decisions
- upstream error classification
- metering and ledger correlation IDs
- delegated identity chain summary when a non-human principal acted on behalf of another subject
- protocol lifecycle summary such as realtime call ID, MCP tool name, or A2A task state timeline

This capsule should be sufficient to explain most customer-visible failures without exposing full prompt content.

Provider-retention guardrail:

- support tooling should not depend on provider-side stored conversations or upstream replay features being enabled
- assume privacy-preserving upstream options such as `store: false` or equivalent are the default posture where available


---


## 38. Diagnostics and Supportability

### 38.1 Request Diagnostics Page

The console should expose a request diagnostics view with:

- request metadata
- selected route
- upstream response status
- retry timeline
- usage summary
- policy decisions
- trace link

### 38.2 Support Tools

Support operators should be able to:

- revoke keys
- quarantine a provider resource
- pause a route target
- issue billing credits/debits with audit trail
- inspect recent failures by tenant/provider/model alias

---
