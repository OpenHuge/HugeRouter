# Implementation Contracts

[Back to Docs Index](../README.md)

This document translates the architecture into coding-facing contracts for the first implementation.

It is intentionally narrower than the broader architecture docs. When an agent needs to choose names, fields, boundaries, or acceptance criteria, this document should win unless a more specific API/schema doc overrides it.

Companion documents:

- [Foundation Risk and Trend Analysis](foundation-risk-and-trend-analysis.md)
- [Event and Message Contracts](event-and-message-contracts.md)
- [Persistence Guidance](persistence-guidance.md)

## 0.1 How to Read This Document

This document intentionally uses three different strengths of guidance:

- **invariant**: should be treated as architecture-preserving and changed only with explicit review
- **recommended default**: the preferred first implementation shape, but alternative designs are acceptable if they preserve the invariants
- **implementation freedom**: areas where agents should optimize for simplicity, leverage existing repo patterns, and avoid unnecessary abstraction

The goal is to reduce ambiguity without freezing the codebase into a single predetermined structure.

## 0.2 Failure Modes This Document Intentionally Counters

This spec is written to reduce common agentic coding and vibe coding failure modes:

- architecture amnesia, where the agent forgets the current system shape and invents parallel structures
- duplicate state ownership, where new stores, caches, or helper layers are added instead of extending the existing source of truth
- side-quest implementation, where the agent creates extra services, abstractions, or frameworks that were not required for the task
- interface-first ambiguity, where types and endpoints exist but behavior, ownership, and failure semantics remain unclear
- log-as-design thinking, where observability is used to compensate for missing domain artifacts or state transitions
- over-generalized future-proofing, where plugin systems, policy matrices, or generic frameworks appear before there is a second concrete use case
- hidden decision drift, where the implementation makes large architectural choices that are not surfaced in the spec or acceptance criteria

## 0.3 Specification Authoring Rules

When adding or revising specs for agent implementation, prefer these rules:

1. define the invariant before defining the extension point
2. identify the single owner of each critical piece of state
3. specify the primary happy path and the 2-3 most important failure paths
4. prefer one clear default over a matrix of configurable options
5. define what should not be created by default, not only what may exist in the future
6. make new services, workers, tables, and crates earn their existence through a concrete operational need
7. describe observable artifacts of success, not only internal abstractions

If a spec section makes it easy for an agent to justify creating a new service, state store, or abstraction without first proving necessity, the section is probably under-constrained in the wrong way.

## 0.4 Additional Guardrails Against Vibe Coding

Coding-facing specs should also avoid these failure-inducing patterns:

- describing component inventories without defining the request or operator outcome they must produce
- using future-facing words such as "generic", "extensible", "pluggable", or "framework" without naming the second concrete use case that requires them
- describing support for "all providers", "all protocols", or "all policies" when the actual V1 boundary is narrower
- mixing current invariants with aspirational roadmap ideas in the same checklist
- leaving completion criteria qualitative when one concrete artifact, API behavior, or testable failure path could make the requirement objective
- introducing placeholder subsystems, empty adapters, or speculative tables purely to make the design look more complete

Preferred rule:

- write specs so that the narrowest correct implementation is obvious

## 1. Primary Goal of V1

The first implementation should prove that HugeRouter can safely admit, route, meter, explain, and audit AI traffic across multiple providers without depending on hidden upstream channels or after-the-fact cost reporting.

V1 is successful if it can do all of the following in a coherent way:

- authenticate a request
- resolve the effective config snapshot
- simulate and select a route
- enforce admission control before upstream execution
- emit a route receipt
- meter usage progressively or finally
- create immutable ledger and audit artifacts
- reconstruct a failure using redacted diagnostics

Additional V1 invariant:

- preserve caller identity, effective acting principal, and delegation chain semantics when the request originates from a non-human or delegated context

## 2. V1 Capability Surfaces

The following artifacts should exist as identifiable concepts in code and storage, even if the first implementation models some of them more lightly than the long-term design:

- `ProviderResource`
- `BudgetPolicy`
- `RoutePolicy`
- `ConfigSnapshot`
- `RouteReceipt`
- `UsageEvent`
- `LedgerEntry`
- `ReplayCapsule`
- delegated identity chain summary

Explicit non-goal for the first implementation:

- a general-purpose agent memory subsystem owned by the gateway

Allowed first-step scope:

- govern memory-bearing traffic, classify memory-related operations, and preserve auditable metadata when upstream systems perform retain/recall/reflect style work

Invariant:

- an implementation should be able to point to where each concept is represented

Implementation freedom:

- these do not all need separate crates, separate tables on day one, or separate API endpoints in the first slice
- a concept may start as a persisted record, a typed struct, or a derived artifact as long as the future boundary remains visible
- concepts should not be split into separate modules merely to mirror the document outline if they still share one lifecycle and one state owner

## 3. Canonical Request Flow

Recommended request sequence:

1. authenticate and authorize
2. resolve caller identity, delegated subject if present, and protocol family
3. resolve effective configuration and produce `config_snapshot_id`
4. normalize protocol-specific input into canonical request semantics
5. expand candidates
6. filter by capability, policy, provenance, and health
7. score candidates
8. run admission control using budget, rate, and concurrency state
9. select target
10. execute upstream call, realtime session, MCP interaction, or A2A task step
11. emit `route_receipt`
12. emit `usage_event`
13. persist ledger and audit follow-up records asynchronously where possible

Invariant:

- upstream execution should not happen before admission control
- if a later stage changes an earlier decision, the system should either prohibit that mutation or re-run the impacted checks and record the transition explicitly
- Chat Completions compatibility must not become the canonical internal OpenAI-family representation; the internal shape should align with Responses-era item and tool semantics
- MCP and A2A should not be collapsed into one generic remote-invocation abstraction without an explicit bridging policy

## 4. Canonical Enums

Recommended default:

- agents should avoid inventing synonyms for the same concept

Implementation freedom:

- internal enum names may differ if they map cleanly to the documented external vocabulary
- external API contracts should converge faster than internal representation if simplification helps implementation

### 4.1 Provider Resource

- `status`: `active | disabled | draining | quarantined | deleted`
- `provenance_class`: `official_api | official_gateway | byo_customer_credential | dedicated_managed_account | shared_brokered_pool | unofficial_client_channel`
- `credential_owner_type`: `platform | tenant | project | partner`
- `health_state`: `healthy | degraded | quarantined | draining | disabled`

### 4.2 Admission Control

- `admission_result`: `admitted | rejected_budget | rejected_rate_limit | rejected_concurrency | rejected_policy | rejected_no_candidate`
- `quota_reserve_strategy`: `none | estimate_then_reserve | fixed_reserve`

### 4.3 Metering and Billing

- `usage_phase`: `reserve | partial | final | release`
- `ledger_entry_type`: `usage_debit | manual_credit | promo_credit | refund_reversal | minimum_fee | reserve_hold | reserve_release`

## 5. Canonical IDs

Recommended default:

- top-level external IDs should be opaque strings with stable prefixes

Recommended prefixes:

- `tenant_`
- `proj_`
- `cred_`
- `prvrsrc_`
- `routepol_`
- `budgetpol_`
- `cfgsnap_`
- `routercpt_`
- `usageevt_`
- `ledger_`
- `replay_`

Implementation freedom:

- internal storage can use UUID, ULID, numeric keys, or other repo-native identifiers
- public prefixing can be added at the API edge if that keeps storage simpler

## 6. First-Class Service Boundaries

### 6.1 `gateway-api`

Recommended ownership:

- auth
- config snapshot resolution
- route simulation and selection
- admission control
- adapter execution
- `route_receipt` emission
- `usage_event` publication

Should avoid owning:

- long-running analytics
- mutable admin state transitions
- billing projection recalculation
- extra orchestration layers that simply wrap existing request flow without changing domain behavior

### 6.2 `control-plane-api`

Recommended ownership:

- CRUD for provider resources, route policies, budget policies
- config snapshot publishing and rollout control
- route simulation API
- route receipt and replay capsule read APIs

Should avoid owning:

- data-plane execution logic
- speculative policy engines that are not yet used by an actual runtime path

### 6.3 `routing-worker`

Recommended ownership:

- background health aggregation
- quarantine transitions
- score input aggregation
- simulation parity helpers if needed

Implementation freedom:

- a thin early implementation may keep some of this logic in-process with the gateway if that reduces moving parts and preserves ownership clarity

### 6.4 `ledger-worker`

Recommended ownership:

- usage event deduplication
- reserve reconciliation
- ledger entry creation
- threshold event emission

Implementation freedom:

- a smaller first slice may persist usage and ledger records synchronously before splitting them into a dedicated worker

## 7. Storage Contracts

### 7.1 PostgreSQL

Recommended default source of truth for:

- provider resource metadata
- route policies
- budget policies
- config snapshots
- route receipts
- usage events index
- ledger entries
- audit events

### 7.2 Redis

May be used for:

- hot quota counters
- route snapshot cache
- ephemeral realtime session state
- circuit breaker state

Invariant:

- Redis loss should degrade the platform safely
- loss of Redis should not silently disable budget or policy enforcement

## 8. API Contract Priorities

If implementation scope is constrained, agents should usually prioritize these external APIs first:

1. provider resource CRUD
2. budget policy CRUD
3. route policy CRUD
4. route simulation
5. public chat completion ingress
6. route receipt retrieval
7. replay capsule retrieval

Implementation freedom:

- a smaller first slice may merge some control-plane surfaces behind fewer endpoints
- retrieval APIs can precede full mutation APIs if that better supports incremental delivery
- OpenAPI completeness can lag one iteration behind runtime behavior as long as the runtime contract is documented and stabilized quickly

Guardrail:

- do not introduce admin APIs for concepts that still have no runtime effect in the first implementation slice
- do not let compatibility-only public APIs dictate the internal request model when a newer canonical provider contract already exists

## 9. Route Receipt Minimum Shape

Recommended minimum external structure:

```json
{
  "route_receipt_id": "routercpt_123",
  "request_id": "req_123",
  "trace_id": "trace_123",
  "config_snapshot_id": "cfgsnap_123",
  "admission_result": "admitted",
  "selected_target": "prvrsrc_123",
  "excluded_targets": [],
  "score_breakdown": {
    "latency": 0.82,
    "cost": 0.66,
    "health": 0.97,
    "trust": 1.0
  },
  "fallback_transitions": []
}
```

Implementation freedom:

- internal storage can denormalize or split this shape
- early implementations may fill some fields lazily, but the external artifact should converge on this envelope

## 10. Replay Capsule Minimum Shape

Invariant:

- the first implementation should not require raw prompt storage for basic supportability

Minimum structure:

```json
{
  "replay_capsule_id": "replay_123",
  "request_id": "req_123",
  "trace_id": "trace_123",
  "route_receipt_id": "routercpt_123",
  "config_snapshot_id": "cfgsnap_123",
  "redaction_tier": "metadata_only",
  "normalized_request_summary": {
    "protocol_family": "openai_chat",
    "model_alias": "reasoning-fast",
    "estimated_prompt_tokens": 12000
  },
  "upstream_error_summary": {
    "code": "upstream_timeout"
  }
}
```

## 11. Acceptance Criteria for Agents

The following are the primary acceptance invariants for agents:

- no request is sent upstream before admission control completes
- every request has stable `request_id` and `config_snapshot_id`
- every routed request has a `route_receipt_id`
- budget and policy rejections return normalized errors
- provenance class is enforced by policy, not treated as free-form metadata
- prompt content is not stored by default in diagnostics tables
- retries do not create duplicate final ledger debits
- route receipts and replay artifacts preserve delegation context when a request was agent-initiated or delegated
- semantic cache is bypassed by default for stateful realtime flows, terminal A2A task transitions, and side-effecting MCP tool operations unless policy explicitly allows it

## 11.1 Suggested Delivery Slices

To avoid overbuilding, agents should prefer thin vertical slices over broad framework work.

Suggested slice order:

1. request ingress + auth + config snapshot resolution
2. route simulation and route selection with one provider family
3. admission control with budget and rate-limit checks
4. route receipt emission
5. usage event + ledger path
6. replay capsule and support read APIs

Each slice should produce a user-visible or operator-visible artifact, not only internal scaffolding.

## 11.2 Suggested Verification Shape

Agents should prefer verification that matches the artifact being introduced.

Recommended defaults:

- domain model slice: typed unit tests for invariants and enum normalization
- routing slice: deterministic fixture-based tests for candidate filtering, exclusion reasons, and selection
- admission control slice: tests covering budget, rate, and concurrency rejection paths
- public API slice: contract tests for normalized error codes and response metadata
- ledger slice: idempotency and duplicate-delivery tests
- replay/support slice: tests proving useful diagnostics can be produced without raw prompt persistence

Implementation freedom:

- these can be unit, integration, or snapshot tests depending on the repo's current maturity
- exact test tool choice matters less than preserving the documented invariants

## 11.3 Common Anti-Patterns to Reject During Implementation Review

Unless a task explicitly calls for them, the following should usually be treated as implementation mistakes:

- adding a new service because the current service "felt crowded" before a concrete scaling or isolation need was shown
- introducing a new source of truth for request, budget, or routing state
- creating generic helper layers that are only used once
- inventing a second identifier vocabulary for the same domain concept
- replacing explicit route or admission artifacts with "we can infer it from logs"
- adding broad configuration knobs before the first operational policy is proven
- implementing supportability by storing raw prompt content by default
- performing large repository-wide refactors while shipping a narrow feature slice
- adding a rules engine, plugin framework, or policy DSL before a second concrete caller forces that abstraction
- shipping placeholder resources or endpoints that are not exercised by the documented request path
- copying upstream protocol fields wholesale into internal domain models without deciding which ones actually affect routing, admission, billing, or diagnostics
- turning semantic cache into a de facto general memory platform without a separate ownership, retention, and policy model
- treating long-running A2A or agent workflow continuation as just another synchronous HTTP retry problem

## 11.4 Backend MVVM-style Layering Rule

This rule applies to future backend work. It does not require proactive refactoring of historical large files.

Invariant:

- new backend product behavior should follow MVVM-style responsibility separation inside the service or crate that owns the feature
- new business logic should not be added directly to oversized service root files such as `services/*/src/lib.rs` or monolithic `store.rs` files
- historical oversized files may remain in place, but a task that must touch related behavior should move only the directly relevant slice into the appropriate module when that can be done safely

Backend mapping:

- **View**: HTTP route and handler code. This layer may authenticate, authorize, parse request inputs, perform cheap syntactic validation, call an application service, and map domain/application errors to HTTP responses.
- **ViewModel**: request/response DTOs, public projections, query result shapes, and redaction-aware response assembly. This layer should not own persistence or business state transitions.
- **Model**: domain records, state machines, invariants, and business rules. This layer should not depend on `axum`, HTTP request types, or `sqlx`.
- **Application**: use case or command service code that coordinates a product flow, owns the transaction boundary, calls repositories, and applies model rules.
- **Repository**: store traits, Postgres adapters, memory adapters, SQL query helpers, and transaction helpers. This layer should not construct HTTP errors or decide user-facing route behavior.

Required default for new backend features:

- create a feature-owned module before adding substantial new code to service roots
- keep handler functions thin enough that their behavior can be summarized as request mapping plus one application call
- keep repository methods focused on data access and persistence invariants
- keep projections and redaction rules explicit rather than leaking storage payloads into public responses
- add tests at the layer that owns the behavior, plus at least one request-level test when a public API contract changes

Recommended module names may include:

- `view.rs`
- `view_model.rs`
- `model.rs`
- `service.rs`
- `repository.rs`

Implementation freedom:

- a small feature does not need all five files on day one
- a module name may differ if the responsibility is still clear
- generated protocol-contract code can remain in protocol crates when that is the established source of truth
- existing legacy functions may be called from new layers when moving them would create unrelated churn

Review triggers:

- a new backend file exceeds 800 lines
- a new handler writes SQL directly
- a repository method returns HTTP-specific errors
- model code imports service-framework or database-driver types
- new product behavior is added to a service root file without a concrete reason

## 11.5 Preferred Agent Behavior

When the spec leaves room for choice, agents should usually prefer:

- extending an existing module over creating a sibling abstraction
- adding one typed record over adding a new framework
- introducing one more explicit field over hiding behavior in free-form metadata
- producing one vertical slice with tests over scaffolding multiple future subsystems
- documenting a deferred concern explicitly instead of prematurely implementing it

## 11.6 Preferred Spec-to-Code Review Questions

Before coding, agents should be able to answer these questions in one or two sentences:

1. what is the smallest user-visible or operator-visible artifact this task must produce
2. which existing module or state owner should absorb the change first
3. which one or two failure paths must be preserved even in the thinnest implementation
4. what tempting abstraction, service, or schema addition is intentionally being deferred

If those answers are unclear, the task is usually not ready for broad implementation work.

## 12. Change Control

### 12.1 Implementation Freedom Zones

Agents should feel free to make pragmatic choices in these areas:

- package and module layout inside a service
- exact repository and table decomposition, as long as the domain concepts remain identifiable
- whether simulation shares code directly with routing or uses a thinner wrapper
- whether route receipts and replay capsules are assembled synchronously or via a follow-up job
- how rich early score breakdowns are, as long as exclusion reasons and selection outcome remain recoverable

### 12.2 Deviation Review Triggers

Any future implementation that wants to:

- skip `route_receipt`
- bypass `config_snapshot_id`
- defer provenance modeling
- make budget enforcement best-effort
- store raw prompt content by default

should be treated as an architecture deviation and reviewed explicitly.
