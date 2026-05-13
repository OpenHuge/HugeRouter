# Event and Message Contracts

[Back to Docs Index](../README.md)

This document defines the minimum message-level contracts that help agents implement asynchronous behavior without overbuilding an event-driven architecture.

It is intentionally narrower than a full messaging design. The goal is to make async boundaries reliable and reviewable while leaving room for pragmatic transport and topic choices.

## 1. How to Use This Document

Use this document when an implementation needs to:

- emit background work from the request path
- persist or publish usage and ledger follow-up artifacts
- fan out audit, threshold, or health changes
- decide whether a state transition deserves a message at all

Preferred rule:

- introduce a message only when another component, process, or retry boundary actually needs it

## 2. Failure Modes This Document Intentionally Counters

This spec is designed to reduce common agent and vibe-coding mistakes around messaging:

- event-everything design, where every internal function call becomes a bus contract
- command-event confusion, where consumers cannot tell whether a message is a fact, a request, or a retry wrapper
- rebuilding critical state from best-effort event streams instead of from the source of truth
- placeholder topics, consumers, or envelopes that exist before any runtime path uses them
- inconsistent correlation fields that make request-level reconstruction impossible
- hidden semantic drift, where similarly named events carry different meanings across producers

## 3. Core Invariants

- critical request-path truth should not depend on asynchronous consumers completing successfully
- every emitted message should be traceable to a stable producer-side record or state transition
- consumers should be able to process duplicate deliveries safely
- messages should describe either:

  - a completed fact that already happened
  - or an explicit work request with clear ownership

- one message should have one primary semantic meaning

Recommended default:

- prefer facts over commands for cross-service notifications

Implementation freedom:

- a smaller first implementation may keep some work in-process and delay introducing external messaging until a retry or scaling boundary is real

## 4. Minimum Envelope

Recommended minimum envelope:

```json
{
  "message_id": "msg_123",
  "message_type": "usage_event.recorded",
  "schema_version": 1,
  "occurred_at": "2026-04-20T00:00:00Z",
  "producer": "gateway-api",
  "trace_id": "trace_123",
  "request_id": "req_123",
  "idempotency_key": "usageevt_123:final",
  "payload": {}
}
```

Recommended default:

- include `trace_id` and `request_id` whenever the message originated from a request lifecycle
- use `schema_version` even if only one version exists initially
- include delegated identity context when the producer acted on behalf of another subject and that fact affects audit, billing, or routing interpretation

Implementation freedom:

- additional fields such as `tenant_id`, `project_id`, `partition_key`, `causation_id`, or `correlation_id` may be added when they help routing or observability
- the physical envelope can be flattened if the bus or queue format prefers that shape

## 5. Event Families Worth Standardizing Early

The first implementation should usually standardize only the event families that directly support runtime behavior or operator workflows:

- `route_receipt.recorded`
- `usage_event.recorded`
- `ledger_entry.created`
- `budget_threshold.exceeded`
- `provider_resource.quarantined`
- `provider_resource.degraded`
- `audit_event.created`
- `config_snapshot.activated`

Guardrail:

- do not standardize a topic family merely because the noun exists in the architecture docs
- do not publish protocol-bridge events until a real MCP-to-A2A or A2A-to-MCP runtime path exists

## 6. Event Semantics

Recommended default:

- event names should read as completed facts

Examples:

- `route_receipt.recorded` means a gateway routing decision was materialized into a durable receipt payload
- `usage_event.recorded` means a usage record was accepted into the authoritative write path
- `ledger_entry.created` means an immutable ledger artifact now exists
- `config_snapshot.activated` means a snapshot became eligible for request selection

Current runtime subject convention:

- `events.route_receipt.recorded` carries `route_receipt.recorded` envelopes from `gateway-api` to the receipt persistence worker

Should avoid:

- event names that hide whether the message is a fact or a command
- event names that encode transport or retry mechanics instead of domain meaning

## 7. Commands and Work Requests

Sometimes a producer needs to ask another component to do work rather than announce a completed fact.

Recommended default:

- keep work-request semantics explicit in naming and ownership

Examples:

- `replay_capsule.build_requested`
- `billing_export.generate_requested`

Guardrail:

- a work request should identify the owner responsible for acting on it
- a work request should not be introduced if the same work can stay safely in-process without losing retry semantics

## 8. Source-of-Truth Relationship

Invariant:

- critical messages should be emitted from a durable source-of-truth write path, not from speculative in-memory state

Recommended default:

- emit messages after the authoritative record is committed, or via an outbox pattern tied to that commit boundary

Implementation freedom:

- early implementations may publish synchronously after commit, use an outbox table, or rely on a repo-native transactional publisher
- exact outbox mechanics do not need to be standardized before more than one producer exists

## 9. Idempotency and Ordering

Recommended defaults:

- consumers should deduplicate by a producer-defined idempotency key or stable artifact ID
- producers should not require global total ordering unless one concrete workflow proves it necessary
- ordering assumptions should be local to a key such as `request_id`, `route_receipt_id`, or `provider_resource_id`

Guardrail:

- do not rely on delivery order alone to preserve billing or admission correctness
- for stateful protocols, do not assume one session or task has exactly one message; model ordering per request, session, or task key explicitly

## 10. Retry and Failure Semantics

The first implementation should make the following semantics explicit:

- whether a failed consumer may retry indefinitely
- whether retry exhaustion produces an operator-visible artifact
- whether the originating request is already considered successful before the consumer finishes

Recommended default:

- request-path success should not be retroactively invalidated by follow-up delivery failures unless the architecture explicitly treats the message as part of a synchronous commit boundary

## 11. Delivery Surface Guidance

Implementation freedom:

- NATS, Redis streams, PostgreSQL-backed outbox polling, or another repo-native durable transport can all satisfy this design
- one topic per event family is not required
- a smaller first slice may multiplex several low-volume event families if consumers still retain clear ownership and filtering

Preferred rule:

- choose the smallest delivery surface that preserves retry visibility, idempotency, and operational clarity

## 12. Review Triggers

The following changes should receive explicit architecture review:

- making request correctness depend on eventual consumer completion
- using the event stream as the only durable record of admission, routing, billing, or audit decisions
- introducing a new bus, topic taxonomy, or message framework before a concrete workload requires it
- publishing raw prompt content or upstream secrets into async payloads by default
- adding event families that have no active producer-consumer path
