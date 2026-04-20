# Persistence Guidance

[Back to Docs Index](../README.md)

This document defines persistence-level guidance for the first implementation.

It is not a fixed database schema. The goal is to preserve auditability, routing correctness, and billing correctness while leaving agents room to choose pragmatic table, repository, and denormalization strategies.

## 1. How to Use This Document

Use this document when an implementation needs to decide:

- what must be durably recorded
- what may be derived or rebuilt
- what deserves immutable history versus mutable current state
- when a new table, collection, or repository boundary is actually justified

Preferred rule:

- persist the smallest set of authoritative records that makes request decisions explainable and billing behavior defensible

## 2. Failure Modes This Document Intentionally Counters

This spec is designed to reduce common persistence mistakes from agentic or vibe-driven implementation:

- table-per-noun design, where every concept in the docs becomes its own persistence artifact on day one
- shadow truth, where projections, caches, or denormalized views quietly become the operational source of truth
- premature normalization that increases joins and coordination before the workload is understood
- unbounded metadata blobs that hide critical behavior from typed review
- mixing mutable operational state, immutable audit history, and analytical projections into the same write model
- storing raw prompt content by default because no smaller support artifact was defined

## 3. Core Invariants

- every critical routing, admission, billing, and audit decision should have one authoritative durable home
- mutable configuration and immutable request-time decisions should remain distinguishable
- projections and caches should be rebuildable from an authoritative record family
- supportability should not require raw prompt persistence by default
- storage loss or replay should not create duplicate final billing effects

## 4. Authoritative Record Families

The first implementation should be able to point to an authoritative durable record for each of these families:

- provider resource state
- route policy state
- budget policy state
- config snapshots
- route receipts
- usage events
- ledger entries
- audit events

Recommended default:

- replay capsules should also be durable once introduced, but they may begin as a thinner derivative artifact instead of a separate fully managed subsystem

Implementation freedom:

- a record family does not need its own table on day one if ownership remains clear
- some families may share one physical store or repository layer in early slices

## 5. Mutable Versus Immutable Data

Recommended default:

- treat provider resources, route policies, and budget policies as mutable current-state records
- treat config snapshots, route receipts, usage events, ledger entries, and audit events as immutable or append-only artifacts

Guardrail:

- do not overwrite the effective request-time decision record in place once it has been exposed to billing, diagnostics, or support flows

## 6. Config and Snapshot Boundary

Invariant:

- requests should be explainable against an immutable `config_snapshot_id` or equivalent request-scoped configuration record

Recommended default:

- mutable control-plane state may evolve freely, but request handling should resolve a stable snapshot before route selection and admission control

Implementation freedom:

- a snapshot may be a dedicated table row, a content-hashed materialization, or another durable representation with stable lookup semantics

## 7. Receipts, Usage, and Ledger Relationship

Recommended default:

- `route_receipt` should explain why a request was or was not routed
- `usage_event` should describe measured or estimated consumption phases
- `ledger_entry` should remain the immutable accounting artifact that affects balances or cost reporting

Guardrail:

- do not collapse all three concepts into one opaque record if it prevents independent reasoning about routing, metering, and accounting

Implementation freedom:

- early implementations may persist these records in the same transaction boundary or repository module
- a thinner slice may derive replay support from `route_receipt` plus `usage_event` before introducing richer capsules

## 8. Derived Data and Projections

The following are usually derived or rebuildable rather than authoritative:

- dashboard summaries
- budget usage aggregates
- provider health rollups
- latency and quality analytics
- routing recommendation caches

Recommended default:

- treat these as projections, not as the only durable record of the underlying behavior

## 9. JSON, Typed Columns, and Metadata

Recommended default:

- use explicit typed fields for values that affect routing, admission, billing, authorization, or operator decisions
- use bounded structured blobs such as JSON for secondary details, diagnostics fragments, or future-compatible payload segments

Guardrail:

- do not hide policy-relevant or billing-relevant behavior inside untyped metadata if those fields must be filtered, joined, or reviewed frequently

Implementation freedom:

- the exact split between typed columns and structured payload blobs may evolve as query patterns become real

## 10. New Table and Repository Creation Rules

Before adding a new table, collection, or repository abstraction, agents should be able to explain:

1. which record family it owns
2. why an existing authoritative store cannot absorb the change cleanly
3. whether the new artifact is authoritative, derived, or cache-like
4. which runtime path or operator task becomes clearer or safer because of the addition

Preferred rule:

- if the only reason for a new table or repository is to mirror the domain outline, it is usually premature

## 11. Retention and Redaction

Invariant:

- retention policy should distinguish between operational truth, audit history, and support diagnostics

Recommended default:

- keep immutable accounting and audit artifacts longer than transient operational caches
- keep replay and diagnostics artifacts redacted by default
- make raw payload retention an explicit exception path rather than a baseline design choice

## 12. Transaction and Write-Path Guidance

Implementation freedom:

- a first slice may use direct transactional writes, a transactional outbox, or a smaller synchronous persistence path followed by async fan-out
- not every durable write needs to happen in one global transaction

Guardrail:

- if a later async step can fail independently, the authoritative earlier record should still make the system state explainable

## 13. Review Triggers

The following should receive explicit review:

- introducing a second source of truth for routing, budget, or ledger state
- making Redis or another cache the only durable holder of policy enforcement state
- persisting raw prompt content by default in operational tables
- creating many low-value tables that have no independent lifecycle or query need
- coupling analytics storage directly into the request hot path without a concrete operational need
