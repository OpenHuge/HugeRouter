# Open Source Reference Patterns

[Back to Docs Index](../README.md)

This document records the architecture patterns we are intentionally borrowing from leading open source products, and how those patterns should influence implementation.

The goal is not to clone any single product. The goal is to avoid inventing first-version architecture in areas where mature projects have already proven better defaults.

The default intake rule for this document is now code-first: a reference only meaningfully influences HugeRouter when repository structure, API resources, manifests, runtime behavior, or official operator-facing documentation make the strength concrete enough to translate into a repository rule.

## 1. Selection Criteria

Reference products were selected because they are strong in one or more of these dimensions:

- control plane and data plane separation
- extension and plugin architecture
- typed configuration and versioning
- observability contracts
- multi-tenant gateway operations
- admin console composition in large monorepos
- concrete source-backed evidence for routing, admission, policy, or runtime behavior

The current reference set is:

- Backstage
- Envoy
- Envoy AI Gateway
- Kong Gateway
- Portkey
- OpenTelemetry
- LiteLLM
- OpenAI Agents SDK
- LangGraph
- Mastra
- Microsoft Agent Framework
- Pydantic AI
- Browser Use
- Hindsight

## 1.1 Code-First Reference Intake

For a reference to change HugeRouter's specification, at least one of the following should be visible in source or official technical material:

- typed resources, manifests, or configuration schemas that show how the capability is modeled
- request-path or control-plane behavior that is explicit enough to copy as a rule
- operational failure behavior that is concrete enough to test
- diagnostics, budget, routing, or auth semantics that are observable rather than marketing-only

When those conditions are not met, the project may still be worth watching, but it should not set HugeRouter defaults yet.

## 1.2 Community Pressure Signals

In addition to named open source references, the architecture is intentionally shaped by repeated operator pain signals visible across public issue trackers and builder communities.

The recurring pressures are:

- routing decisions that are difficult to explain after incidents
- rate-limit handling that fails under provider-specific or sub-minute enforcement windows
- budgets that are reported after the fact instead of enforced before spend occurs
- hidden or low-trust upstream brokerage that creates legal and operational risk
- observability designs that over-collect prompt content and create new privacy problems
- multi-tenant products that lack configuration snapshotting, staged rollout, and fast rollback

These are not implementation details. They are the real-world constraints that separate a demo gateway from a production-grade AI traffic platform.

## 2. Backstage: Package-Level Plugin Boundaries

Backstage is useful as a reference for frontend and monorepo composition, not because our product is a developer portal, but because it proves that large internal products stay healthier when extensions are packaged as isolated units with explicit app-level assembly.

Patterns to adopt:

- treat plugins and extensions as separate packages rather than as folders hidden inside one large app
- keep app assembly explicit, with the app deciding which extensions are mounted and where
- expose extension points through stable handles and neutral route contracts rather than direct imports into feature internals
- prefer package-local ownership and isolated development workflows for shared UI and feature surfaces

Application to this repository:

- `packages/ui-kit` and future console extension packages should behave like product-owned modules, not app-internal dumping grounds
- `apps/console-web` remains the composition root for route mounting, shell wiring, and app-level providers
- route and navigation integration should happen through explicit composition contracts, not package-to-package deep imports

## 3. Envoy: Ordered Filter Chains and Typed Extension Config

Envoy is the best reference for request-path composition discipline.

Patterns to adopt:

- treat the request path as an ordered filter or middleware chain
- make extension configuration typed and versioned
- assume ordering matters and document that ordering as part of the design, not an implementation detail
- make cross-cutting data shareable through request context rather than through global state

Important caution drawn from Envoy:

- route mutation after authorization is dangerous
- if later middleware can change route selection, the architecture must either prevent that after auth or force re-evaluation of policy

Application to this repository:

- gateway middleware order is part of the architecture contract
- adapter manifests and plugin manifests should behave like typed extension configs, not free-form blobs
- route re-selection after policy enforcement must be tightly controlled and auditable

## 4. Envoy AI Gateway: AI-Specific Logic as Extensible Data-Plane Processing

Envoy AI Gateway is a valuable emerging reference because it separates general proxy concerns from AI-specific processing instead of re-implementing everything in one controller.

Patterns to adopt:

- keep AI-specific traffic logic isolated from generic transport and proxy responsibilities
- allow specialized processing stages to enrich routing, validation, auth, and token-aware rate limiting without collapsing all concerns into one component
- preserve a clean handoff between control-plane configuration generation and data-plane execution

Application to this repository:

- `runtime-composition` should own AI-specific pipeline assembly without turning every service into a monolith
- token-aware policy, provider fallback context, and protocol normalization should live in explicit pipeline stages
- generic infrastructure concerns such as TLS, listener lifecycle, or baseline HTTP serving should remain platform concerns, not adapter concerns

## 5. Kong Gateway: Hybrid Control Plane and Data Plane Operations

Kong is the strongest reference for practical CP/DP operations in a commercial-style gateway.

Patterns to adopt:

- control plane manages config and administration; data plane serves traffic
- data plane should continue serving with the last valid configuration when control plane connectivity is lost
- CP/DP communication should be mutually authenticated
- version compatibility between control plane and data plane must be explicit, documented, and enforced

Application to this repository:

- route config, policies, credentials metadata, and limits should be published to the hot path as versioned snapshots
- the gateway request path must not depend on a live round-trip to the control plane for ordinary request execution
- when control plane is degraded, data plane should fail stale-aware, not fail open or thrash on partial config
- service-to-service trust between planes should support mTLS-first deployment modes

## 6. OpenTelemetry: Semantic Contracts Before Dashboard Sprawl

OpenTelemetry is the reference for observability governance.

Patterns to adopt:

- define semantic conventions before teams invent free-form telemetry names
- centralize shared attribute names, event names, and metric units
- treat telemetry shape as an API contract
- document unstable attributes clearly and keep them separate from stable contracts

Application to this repository:

- `crates/telemetry` should own shared attribute keys and event naming
- route decisions, adapter execution, billing events, and policy outcomes should use typed semantic fields
- unstable AI-specific attributes should be clearly namespaced and versioned rather than mixed into stable telemetry casually

## 7. LiteLLM: Virtual Keys, Budgets, and Practical Multi-Provider Routing

LiteLLM is the most relevant open source reference for day-to-day AI gateway ergonomics.

Patterns to adopt:

- separate consumer-facing gateway keys from upstream provider secrets
- support project- and key-level budgets as first-class controls
- keep routing strategies explicit, inspectable, and configurable
- normalize provider differences while preserving enough provider-specific metadata for diagnostics

Application to this repository:

- the control plane should issue virtual gateway credentials rather than exposing upstream keys
- routing policy should support explicit strategies such as priority, least-busy, latency-aware, or cost-aware selection
- budget controls belong in the product model early, even if advanced forecasting comes later
- northbound compatibility can start with OpenAI-shaped APIs, but the IR must remain broader than a single provider format

Additional signal from recent LiteLLM direction:

- one gateway can expose model, agent, MCP, and A2A surfaces side by side
- that convenience should not erase the distinction between compatibility APIs and canonical internal semantics

Architectural implication:

- HugeRouter should welcome multiple northbound compatibility surfaces, but route receipts, policy, metering, and diagnostics should still anchor on canonical domain artifacts rather than whichever compatibility endpoint happened to be called

## 8. OpenAI Agents SDK: Sessions, Guardrails, and Traceable Runs

The OpenAI Agents SDK is a useful reference because it treats sessions, tracing, human-in-the-loop, handoffs, and realtime agents as core runtime concepts rather than side features.

Patterns to adopt:

- keep request, session, task, and operation scopes distinct
- treat tracing as a runtime contract, not just a logs concern
- model approval and human intervention as normal lifecycle steps
- keep handoffs and delegation visible instead of flattening them into one opaque model call

Application to this repository:

- HugeRouter should preserve separate identifiers and diagnostics context for request, runtime session, delegated task, and governed operation
- approval checkpoints should be first-class artifacts, not ad hoc flags
- route receipts, replay capsules, and audit events should be able to explain agent handoff and session context without storing full transcript content

## 9. LangGraph and Microsoft Agent Framework: Durable Agent State and Long-Running Work

Recent agent runtimes such as LangGraph and Microsoft Agent Framework have reinforced one design lesson: multi-agent work should be modeled as durable, resumable stateful execution, not as a series of fragile best-effort RPC hops.

Patterns to adopt:

- treat long-running agent and A2A work as explicit state machines with stable identifiers
- preserve pause, resume, retry, and human-in-the-loop intervention as first-class lifecycle concepts
- keep task continuation and task terminal-state handling explicit rather than inferring them from chat history or logs
- make orchestration visibility part of the runtime contract, not an afterthought

Application to this repository:

- A2A task handling should be modeled as a durable lifecycle with immutable terminal states
- route receipts, replay capsules, and async envelopes should preserve enough context to explain task continuation and delegated execution
- the gateway should not assume agent work is synchronous simply because one protocol hop uses HTTP
- external agents, third-party tools, and non-local runtimes should remain explicit trust boundaries rather than disappearing into generic "successful delegation"

## 10. Mastra and Pydantic AI: Typed Capabilities, Memory, and Human Oversight

Recent agent frameworks such as Mastra and Pydantic AI have converged on several useful defaults:

- typed capabilities and strongly validated IO
- memory and context treated as explicit subsystems
- human approval or interruption as part of normal execution, not exceptional control flow
- MCP and A2A treated as integration surfaces rather than hidden framework internals

Patterns to adopt:

- keep typed schemas at the package and protocol boundary
- model human approval and delegated continuation as expected workflow states
- expose memory-relevant metadata and policy hooks without turning the gateway into the canonical memory owner for every agent

Application to this repository:

- HugeRouter should govern memory traffic, retention class, and policy eligibility where memory-bearing systems are involved
- HugeRouter should not become a general-purpose agent memory platform by default
- the gateway should preserve typed protocol contracts and approval states so operator tooling can reason about them cleanly
- typed policy and protocol boundaries should beat free-form metadata whenever an operation affects routing, approval, or retention

## 11. Browser Use: High-Side-Effect Tools and Browser-Bound Identity

Browser Use is a strong cautionary reference because it makes three realities explicit: browser agents carry authenticated session state, custom tools are easy to add, and browser actions can mutate external systems in ways that are much more sensitive than plain inference.

Patterns to adopt:

- classify browser and tool operations by side-effect level instead of treating them as one generic tool category
- treat browser-authenticated state, cookies, and synced profiles as sensitive delegated credentials
- keep external-action governance visible even when the runtime makes automation feel ergonomic

Application to this repository:

- HugeRouter should distinguish `browser_read` from `browser_write`, and low-side-effect tool calls from material external mutations
- approval policy should be able to pause or isolate browser and tool operations before execution continues
- replay and audit artifacts should preserve that browser-authenticated or delegated credentials were involved without exposing the underlying secret material

## 12. Hindsight: Memory Is Not Just Retrieval

Newer open source memory systems such as Hindsight show a shift from "chat history recall" to a richer retain/recall/reflect model with multiple retrieval strategies and explicit memory classes.

Patterns to adopt:

- assume memory-bearing agents may distinguish between raw events, learned summaries, and durable observations
- keep per-user and per-agent memory isolation explicit
- expect memory systems to emit their own identifiers, provenance, and lifecycle metadata

Application to this repository:

- the gateway should treat memory systems as governed upstream resources, not flatten them into ordinary vector search
- policy and observability should be able to distinguish ordinary inference calls from memory retain/recall/reflect style operations
- semantic cache should remain distinct from agent memory, even if both use embeddings or similarity search under the hood

## 13. OpenTelemetry Collector: Pipeline Components and Distribution Discipline

OpenTelemetry has evolved beyond naming guidance; the Collector ecosystem now reinforces a strong component model of receivers, processors, exporters, connectors, and extensions.

Patterns to adopt:

- treat telemetry pipelines as composable components with clear responsibilities
- separate ingest, transform, and export concerns
- keep internal telemetry pipelines distributable and reducible, rather than forcing one giant always-on collector shape

Application to this repository:

- HugeRouter should think about diagnostics and telemetry pipelines the same way it thinks about request pipelines: typed stages, explicit ordering, and clear handoff semantics
- the platform should preserve room for connector-style fan-out from traces to metrics or support artifacts without making those derived views the source of truth
- observability deployment should support a minimal local distribution first and richer production distributions later

## 14. 2026 Peer-Derived Priorities We Should Copy First

Current repository reality matters when choosing what to copy next.

Today the codebase already proves:

- one synchronous OpenAI-compatible gateway path is real
- one real OpenAI upstream adapter is real
- control-plane auth, seed data, config snapshot reads, and route simulation are more mature than most other subsystems
- workers, streaming, realtime, durable usage persistence, and full observability pipelines are still mostly scaffolded or planned

Because of that baseline, the most valuable peer-derived priorities are not more abstract principles. They are concrete product behaviors that close the current gap between the repository's real runtime and the broader specification.

The current spec deepening pass for those priorities lives primarily in:

- `routing-system.md`
- `policy-system.md`
- `protocol-ir-and-protocols.md`
- `provider-adapter-system.md`
- `multi-tenancy-and-configuration.md`
- `metering-ledger-pricing.md`

### 14.1 LiteLLM and Kong: Routing, Fallback, and Budget Admission Must Be Concrete

What current peer projects make clear:

- routing strategy names should be product-level concepts, not hidden implementation detail
- fallback order, retry eligibility, cooldown, and exclusion reasons should be inspectable
- budget and rate checks belong in admission control before upstream spend occurs
- response headers and route receipts should explain routing and limit behavior consistently

Translation for HugeRouter:

- `routing-system.md` should define named strategy families and their observable decision fields
- route receipts should capture retry and fallback transitions as first-class artifacts, not only debug headers
- budget, quota, and concurrency rejection states should be specified as terminal admission outcomes, then implemented on the hot path
- rate-limit and budget headers should become part of the external behavior contract once the gateway exposes them

### 14.2 Envoy AI Gateway: AI Traffic Features Should Be Typed Resources, Not Handler Special Cases

What current peer projects make clear:

- AI routing, MCP routing, quota, and inference-target selection stay maintainable when modeled as typed resources
- control-plane resources should describe intent; data-plane code should execute validated snapshots
- endpoint picker and protocol-specific behavior should remain explicit extension points

Translation for HugeRouter:

- MCP, A2A, northbound protocol compatibility, and provider target selection should converge on typed manifests and typed route resources
- `protocol-ir`, `provider-traits`, and config snapshot schemas should remain the canonical place where those contracts live
- new AI protocol surfaces should not be introduced first as ad hoc flags in service handlers

### 14.3 Kong Hybrid Mode: CP/DP Failure Behavior Must Be Specified Before Scale

What current peer projects make clear:

- a gateway becomes operationally trustworthy only when CP/DP disconnect behavior is explicit
- data planes need last-known-good snapshot behavior rather than live dependency on mutable control-plane state
- version and extension compatibility rules need to be part of the platform contract

Translation for HugeRouter:

- `overview.md` and `multi-tenancy-and-configuration.md` should treat stale-aware snapshot serving as a contract, not an aspiration
- compatibility between config snapshot shape, adapter manifests, and runtime binaries should be documented as a release gate
- configuration publication, rollback, and stale snapshot diagnostics should be testable behavior

### 14.4 Portkey: Policy Composition and Failure Domains Need First-Class Traceability

What current peer projects make clear:

- retries, fallbacks, load balancing, guardrails, and conditional routing are most useful when composable
- operators need to see which nested rule fired, not merely that a request failed
- pricing catalogs and routing rules should be independent artifacts, even when the product combines them

Translation for HugeRouter:

- policy and routing specs should distinguish selection, guardrail, admission, retry, and fallback stages explicitly
- nested policy outcomes should remain traceable in route receipts and redacted diagnostics
- pricing catalogs should remain separable from route policy definitions and from immutable usage facts

## 15. What We Adopt Immediately

The following are now considered implementation defaults because peer projects make the benefit concrete and the current HugeRouter repository needs them soon:

- package and plugin boundaries are explicit and app-first
- request-path ordering is part of the architecture contract
- manifests and extension configs are typed and versioned
- CP/DP separation is real, not merely conceptual
- telemetry naming is governed centrally
- gateway keys, upstream secrets, budgets, and routing strategies are first-class domain concepts
- routing strategy, exclusion, retry, fallback, and admission outcomes must become explicit external or internal artifacts rather than incidental logs
- long-running agent and A2A work is treated as durable lifecycle state, not just request/response traffic
- request, session, task, and operation identifiers remain distinct when governance or diagnostics require it
- approval checkpoints are a normal control outcome for high-risk tool, browser, memory-write, or delegation flows
- semantic cache and agent memory are treated as related but distinct concerns
- compatibility endpoints do not define canonical internal semantics by themselves

## 16. What We Deliberately Do Not Copy

We are not copying these parts directly:

- Backstage's full plugin runtime and developer portal mental model
- Envoy's full xDS control plane complexity
- Kong's exact deployment topology or operational product model
- LiteLLM's Python implementation model or OpenAI-format-first worldview as the long-term platform boundary
- OpenAI Agents SDK as the gateway's internal runtime framework
- LangGraph or Microsoft Agent Framework as the application's orchestration runtime
- Mastra or Pydantic AI as the gateway's internal service framework
- Browser Use's browser runtime as a built-in gateway execution substrate
- Hindsight's memory-system internals as a requirement for the gateway's own storage model

These projects are reference patterns, not product templates.

## 17. Development Implication

Because these reference patterns are now captured explicitly, implementation should assume:

- shared packages and crates are contracts, not convenience folders
- routing and auth order must be testable
- CP/DP compatibility, config snapshotting, and telemetry semantics are build-time concerns, not cleanup tasks for later
- task lifecycles, delegation, and human intervention states should be modeled explicitly once agentic execution enters scope
- high-side-effect browser, tool, memory-write, and delegation operations should be governable without inventing a full agent runtime inside the gateway
- semantic cache should not quietly expand into a generic memory subsystem
- spec changes should cite the source-backed reference strength they are adopting when a comparable external pattern exists
- spec changes should distinguish whether the behavior is already implemented, bootstrap-only, or still planned in this repository
- feature work that breaks these patterns should be treated as an architecture change, not a small refactor
