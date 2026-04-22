# Foundation Risk and Trend Analysis

[Back to Docs Index](../README.md)

This document connects HugeRouter's product direction with known pressure points from mature open source systems and the current AI gateway market. It is intended to guide bottom-layer architecture decisions before implementation choices become difficult to reverse.

HugeRouter is not a generic API proxy. It is a protocol-native AI traffic platform. The architecture must therefore optimize for explainable routing, budget-safe admission control, trustworthy provider capacity, redaction-first diagnostics, and protocol evolution across OpenAI-compatible APIs, native provider APIs, Realtime transports, and MCP.

Reference anchors checked for this analysis:

- Backstage frontend plugin and app composition documentation
- Envoy extension configuration, Envoy AI Gateway repository and data-plane documentation
- Kong Gateway repository plus hybrid mode and AI gateway documentation
- OpenTelemetry GenAI semantic conventions
- LiteLLM repository plus routing and gateway documentation
- Portkey gateway repository and fallback or routing documentation where the behavior is concrete enough to evaluate

## 1. Product Characteristics

HugeRouter has five defining characteristics:

- it serves multi-tenant AI traffic where requests may be expensive, long-lived, streaming, or agentic
- it must separate control-plane mutability from data-plane hot-path execution
- it must normalize provider differences without flattening every request into one lossy schema
- it must meter and bill from immutable usage facts instead of mutable summaries
- it must support self-hosted and managed deployments without weakening security defaults

The bottom-layer design should therefore avoid three common traps:

- building a simple OpenAI-compatible forwarding panel and later trying to retrofit policy, billing, and diagnostics
- creating a large generic plugin framework before there are stable contracts and second concrete use cases
- treating observability, budget controls, or provenance checks as optional add-ons instead of admission-path concerns

## 2. Open Source Lessons

The adoption rule for this section is code-first: lessons should be derived from concrete source-backed behavior, typed resources, or operator-facing technical documentation that exposes how the system actually behaves.

### 2.1 Backstage

Backstage shows that large products remain maintainable when extension boundaries are explicit and app composition is centralized.

Adopt:

- package-level ownership for console features and shared UI
- explicit app-level route and navigation composition
- stable extension handles instead of deep imports between feature internals

Avoid:

- copying Backstage's full plugin runtime before HugeRouter has a mature extension ecosystem
- allowing `apps/console-web` to become the hidden owner of product domain logic

### 2.2 Envoy and Envoy AI Gateway

Envoy's strongest lesson is ordered request-path discipline. Envoy AI Gateway adds the newer lesson that AI-specific processing should extend the data plane without collapsing generic proxying, policy, and provider logic into one monolith.

Adopt:

- a typed, ordered pipeline for auth, config snapshot resolution, policy, routing, admission, adapter execution, metering, and receipt emission
- extension config that is versioned and validated before it reaches the hot path
- explicit prevention or re-evaluation when a later stage tries to mutate an earlier security-sensitive decision

Avoid:

- switch-heavy protocol and provider logic inside service handlers
- route mutation after authorization without a recorded re-check
- coupling generic listener lifecycle to provider adapter behavior

### 2.3 Kong Gateway

Kong's hybrid mode proves the operational value of a real control-plane/data-plane split.

Adopt:

- data planes serve from last-known-good config snapshots
- control-plane publishing is versioned, staged, and rollbackable
- CP/DP compatibility is checked explicitly
- service-to-service plane communication supports mTLS-first deployments

Avoid:

- data-plane requests that require live control-plane round trips for ordinary execution
- partial config pushes that can leave policy, routes, and provider resources out of sync
- hidden dependencies on database access from data-plane nodes

### 2.4 OpenTelemetry

OpenTelemetry's GenAI semantic conventions reinforce that telemetry shape is an API contract, not a dashboard afterthought. They also emphasize that prompts, inputs, and outputs are sensitive and should not be recorded by default.

Adopt:

- a shared telemetry semantic registry owned by a dedicated crate
- stable fields for request, route, policy, adapter, usage, and ledger correlation
- opt-in payload capture with redaction, retention, and access policy

Avoid:

- service-local telemetry keys that drift across crates
- default raw prompt retention
- using unstructured logs as the only route explanation mechanism

### 2.5 LiteLLM

LiteLLM demonstrates practical AI gateway ergonomics: virtual keys, provider normalization, budget controls, and model routing. Its limitation as a long-term reference is that an OpenAI-format-first worldview can become too narrow for MCP, Realtime, and native provider capabilities.

Adopt:

- virtual gateway credentials instead of exposing upstream provider secrets
- tenant, project, key, and model budget controls as first-class policy objects
- explicit routing strategies with inspectable diagnostics
- per-request cost visibility rather than provider-level aggregate-only reporting

Avoid:

- irreversible coupling to OpenAI-shaped request semantics
- cost reporting that happens only after upstream spend is already committed
- provider abstraction that hides important provenance or capability differences

## 3. Current Market and Operator Pain Points

The strongest recurring pain points for AI gateways are:

- route decisions are hard to explain after incidents
- retries amplify cost by replaying large context without clear per-request accounting
- provider rate limits use different RPM, TPM, concurrency, and sub-minute windows
- budgets are often reported after spend rather than enforced before admission
- streaming and Realtime sessions need sticky routing, reserve accounting, and graceful teardown semantics
- MCP introduces tool, identity, consent, and transport boundaries that simple HTTP proxy designs do not capture
- raw prompt logging creates privacy, compliance, and support-access risk
- provider brokerage can obscure whether capacity is official, customer-owned, partner-managed, or unsafe
- control-plane outages can break traffic when hot-path configuration is not snapshotted
- multi-tenant configuration changes are difficult to stage, simulate, and roll back

These pain points define the architecture floor. A feature is not production-grade until it has a route receipt, admission-control outcome, telemetry correlation, failure behavior, and rollback path.

## 4. Future Core Trends

### 4.1 Protocol-Native AI Traffic

The gateway must support OpenAI-compatible APIs while modeling Responses-era semantics, Anthropic and Gemini-native structures, Realtime WebRTC/WebSocket flows, and MCP as first-class protocol families.

Design implication:

- `protocol-ir` owns semantic request and response representation
- protocol handlers parse into IR before routing and policy
- provider-specific extensions remain namespaced and non-security-critical unless promoted into typed capabilities

### 4.2 Agentic and Tool-Oriented Traffic

Agent workloads increase the importance of tool permissions, trace continuity, session state, and policy decisions that span more than one HTTP request.

Design implication:

- MCP and tool calls must carry identity, authorization, and trace context
- replay capsules should reconstruct policy and route choices without storing raw tool payloads by default
- session affinity and stateful reconnection must be part of routing policy for long-lived traffic

### 4.3 Cost and Quota Admission Control

AI requests can be expensive and retry-heavy. Budget enforcement after the fact is not sufficient.

Design implication:

- budgets, quotas, and provider-native rate windows are pre-admission inputs
- expensive or long-lived sessions use reserve and release events
- every rejection still emits a route receipt with deterministic reason codes

### 4.4 Trust and Provenance Filtering

Provider resources are not interchangeable. Official APIs, customer credentials, dedicated managed accounts, shared brokered pools, and unofficial channels carry different legal and reliability risks.

Design implication:

- `ProviderResource.provenance_class` is a routing and policy input
- unsafe provenance classes are denied by default in production and enterprise modes
- route receipts include trust-class filtering and exclusion reasons

### 4.5 Redaction-First Observability

GenAI observability is moving toward standardized spans and metrics, but payload capture remains sensitive and expensive.

Design implication:

- payload capture is disabled by default
- telemetry records metadata, fingerprints, usage, and decision artifacts first
- sealed payload capture requires explicit tenant policy, retention, access control, and audit events

### 4.6 Control-Plane/Data-Plane Snapshotting

Production gateways must continue serving predictable traffic when the control plane or database is degraded.

Design implication:

- data-plane services consume immutable `ConfigSnapshot` artifacts
- snapshots include compatible route, policy, provider, budget, and telemetry semantic versions
- rollout supports canary, validation, last-known-good fallback, and fast rollback

## 5. Bottom-Layer Architecture Design

### 5.1 Canonical Hot-Path Pipeline

The data-plane pipeline must keep this order:

1. authenticate gateway credential
2. authorize tenant, project, key, and endpoint scope
3. resolve effective `ConfigSnapshot`
4. parse protocol request into `AiRequestIr`
5. classify capabilities, modalities, tool usage, and session requirements
6. expand route candidates
7. filter by capability, policy, region, provenance, and health
8. score candidates
9. enforce budget, rate, quota, and concurrency admission control
10. select route and bind session affinity where required
11. execute adapter pipeline
12. meter reserve, partial, final, and release usage phases
13. emit `RouteReceipt`, `UsageEvent`, audit event, and telemetry correlation

Invariant:

- no upstream execution before admission control
- no route target mutation after admission unless policy and admission are re-run and the transition is recorded
- every terminal path emits an explainable artifact, including rejected requests

### 5.2 Core Domain Artifacts

The first durable architecture layer should stabilize these concepts before expanding feature count:

- `ProviderResource`: upstream capacity, capability, credential reference, provenance, health, region, and status
- `GatewayCredential`: consumer-facing virtual key with tenant, project, budget, and policy scopes
- `ConfigSnapshot`: immutable data-plane configuration bundle with compatibility metadata
- `RoutePolicy`: candidate matching, filtering, strategy, retry, fallback, residency, provenance, and stickiness rules
- `BudgetPolicy`: hard and soft spend limits, reserve strategy, refill or duration rules, and model-level caps
- `RouteReceipt`: route explanation, candidate exclusions, scores, selected target, retry chain, and policy snapshot IDs
- `UsageEvent`: immutable usage fact with idempotency key and usage phase
- `LedgerEntry`: accounting fact derived from usage events, never edited in place
- `ReplayCapsule`: redacted support bundle that links route, policy, usage, audit, and trace artifacts

### 5.3 Config Snapshot Contract

A `ConfigSnapshot` should be the only ordinary data-plane view of mutable control-plane state.

Minimum contents:

- snapshot ID, version, build timestamp, issuer, and signature metadata
- compatible data-plane version range
- route policies and strategy weights
- provider resources and capability manifests
- budget policies and rate limit windows
- credential scope metadata without raw provider secrets
- telemetry semantic version
- rollout metadata and previous snapshot pointer

Validation rules:

- reject snapshots with dangling provider, policy, or budget references
- reject snapshots that require unknown protocol handlers, adapters, or telemetry semantic versions
- load new snapshots atomically
- retain last-known-good snapshot when the latest snapshot is invalid or unavailable

### 5.4 Routing and Admission Engine

The routing engine should be deterministic for the same request, snapshot, and health inputs.

Core requirements:

- candidate expansion separates logical model aliases from physical provider resources
- filtering produces reason codes, not only booleans
- scoring outputs dimension-level contributions for latency, cost, health, quality, residency, and trust
- admission control reserves budget and quota before execution where policy requires it
- retries are bounded, classified, and cost-aware
- fallback never bypasses provenance, residency, budget, or capability filters

### 5.5 Provider Adapter Boundary

Provider adapters should not own routing, budget policy, tenant authorization, or raw credential distribution.

Adapter responsibilities:

- render provider-native requests from IR
- execute provider calls or sessions
- normalize provider errors
- extract usage and provider annotations
- expose capability and configuration manifests

Adapter non-responsibilities:

- choosing whether a tenant is allowed to use a provider
- reading raw control-plane mutable state
- inventing telemetry fields outside the semantic registry
- persisting ledger entries directly

### 5.6 Ledger and Metering Boundary

Usage and billing correctness must be event-first.

Rules:

- `UsageEvent` is immutable and idempotent
- `LedgerEntry` is append-only
- projections are repairable and replayable
- reserve holds and releases are explicit entries
- retries and fallback attempts are visible in per-request cost records
- billing code must never infer final cost only from logs or provider aggregates

### 5.7 Observability and Support Boundary

Every customer-visible failure should be explainable without privileged access to raw prompt content.

Required correlation keys:

- request ID
- trace ID
- tenant ID
- project ID
- gateway credential ID or prefix
- config snapshot ID
- route policy ID
- provider resource ID
- route receipt ID
- usage event ID
- ledger entry IDs where available
- replay capsule ID where created

Default retention model:

- metadata and route receipts: retained for diagnostics and billing explanation
- usage and ledger records: retained according to accounting and compliance policy
- sealed payloads: disabled by default and separately governed when enabled

## 6. Potential Problems and Avoidance Rules

### 6.1 Architecture Drift

Problem:

- services slowly invent parallel ownership for config, policy, routing, or usage facts

Avoidance:

- single source of truth per domain concept
- new services require a distinct scaling, isolation, lifecycle, or ownership reason
- ADR required for changes that alter hot-path order, data ownership, or compatibility rules

### 6.2 Unsafe Provider Abstraction

Problem:

- all provider resources are treated as equivalent because they share a model name

Avoidance:

- require provenance, capability, region, credential owner, and health metadata on provider resources
- deny unsafe provenance classes by default
- expose trust-class decisions in route receipts

### 6.3 Cost Overrun

Problem:

- retries, long contexts, streaming sessions, and fallback paths spend more than expected

Avoidance:

- admission reserve before expensive calls
- retry budget and max marginal cost in route policy
- per-attempt usage and cost attribution
- deterministic rejection when budget or quota is exhausted

### 6.4 Observability Data Leakage

Problem:

- raw prompts, tools, secrets, or regulated data leak through traces, logs, or support exports

Avoidance:

- default payload capture off
- redaction helpers centralized and tested
- sealed payload capture gated by tenant policy and audited access
- telemetry semantic registry separates stable metadata from sensitive content

### 6.5 CP/DP Coupling

Problem:

- data-plane availability depends on control-plane database or live admin APIs

Avoidance:

- data plane reads only signed, immutable snapshots in the ordinary hot path
- last-known-good snapshot survives control-plane outages
- snapshot compatibility and atomic load are tested

### 6.6 Plugin Overreach

Problem:

- the project builds a dynamic plugin runtime before stable extension contracts exist

Avoidance:

- compose statically first through registries, manifests, and composition roots
- dynamic loading remains a future capability after contracts and sandbox rules are proven
- every extension point needs a second concrete use case before becoming generic

### 6.7 Protocol Narrowing

Problem:

- OpenAI compatibility becomes the internal product model and blocks native, Realtime, or MCP capability

Avoidance:

- route and policy operate on IR capability contracts, not raw provider request fields
- OpenAI-compatible endpoints are ingress compatibility surfaces, not the canonical internal model
- protocol-specific metadata stays namespaced until promoted into typed IR fields

## 7. Implementation Priorities

### 7.1 Foundation Slice

Build first:

- typed domain IDs and enums in `core-domain`
- IR request and response primitives in `protocol-ir`
- provider manifest and adapter traits in `provider-traits`
- static registry and composition helpers in `plugin-registry` and `runtime-composition`
- route candidate, route receipt, and admission result models
- telemetry semantic constants
- a source-review matrix that maps borrowed external strengths to HugeRouter rules and labels each one as implemented, bootstrap-only, or planned

Exit criteria:

- a request can be parsed, routed, admitted or rejected, and explained with stable IDs even if only one provider adapter exists
- the first implementation slice can explain which peer-proven behaviors it copied and which ones are still planned

### 7.2 Gateway Slice

Build next:

- OpenAI-compatible ingress as the first northbound compatibility surface
- one native official provider adapter
- API key authentication with virtual gateway credentials
- basic budget and rate admission control
- route receipt emission for success and rejection
- usage event emission with idempotency

Exit criteria:

- no upstream request can happen without auth, config snapshot, policy, route, and admission artifacts

### 7.3 Operational Slice

Build after the gateway slice:

- config snapshot publishing and rollback
- health aggregation and quarantine transitions
- route simulation API
- redacted request diagnostics UI
- ledger worker idempotency and projection repair path
- synthetic probes for provider and protocol paths

Exit criteria:

- operators can answer why a request routed, why it failed, how much it cost, and which snapshot caused the behavior

## 8. Non-Negotiable Design Checks

Before merging a feature that touches the gateway path, confirm:

- Does it preserve the canonical hot-path order?
- Does it use the effective `ConfigSnapshot` instead of direct mutable control-plane reads?
- Does it emit or update a route receipt for every terminal outcome?
- Does it enforce budget, quota, and provenance before upstream execution?
- Does it avoid raw prompt logging by default?
- Does it use shared telemetry fields?
- Does it keep provider-specific behavior inside adapter or protocol boundaries?
- Does it make retries and fallback visible in cost and diagnostics?
- Does it remain deterministic under the same snapshot and health inputs?
- Does it have a rollback or last-known-good behavior when config changes fail?
- Does the spec cite the relevant external reference strength when one exists, or explicitly justify the novel behavior?
- Does the spec label the behavior as implemented, bootstrap-only, or planned in this repository?

## 9. External References and Source Repositories Reviewed

- Backstage frontend plugins: <https://backstage.io/docs/frontend-system/architecture/plugins>
- Backstage app architecture: <https://backstage.io/docs/next/overview/architecture-overview/>
- Envoy extension configuration: <https://www.envoyproxy.io/docs/envoy/latest/configuration/overview/extension.html>
- Envoy AI Gateway repository: <https://github.com/envoyproxy/ai-gateway>
- Envoy AI Gateway data plane: <https://aigateway.envoyproxy.io/docs/concepts/architecture/data-plane/>
- Envoy AI Gateway MCP capability: <https://aigateway.envoyproxy.io/docs/capabilities/mcp/>
- Kong Gateway repository: <https://github.com/Kong/kong>
- Kong Gateway hybrid mode: <https://developer.konghq.com/gateway/hybrid-mode/>
- Kong AI Proxy Advanced: <https://developer.konghq.com/plugins/ai-proxy-advanced/>
- OpenTelemetry GenAI semantic conventions: <https://opentelemetry.io/docs/specs/semconv/gen-ai/gen-ai-spans/>
- LiteLLM repository: <https://github.com/BerriAI/litellm>
- LiteLLM routing docs: <https://docs.litellm.ai/docs/routing>
- LiteLLM open source gateway overview: <https://www.litellm.ai/oss>
- Portkey gateway repository: <https://github.com/Portkey-AI/gateway>
- Portkey AI gateway fallbacks: <https://portkey.ai/docs/product/ai-gateway/fallbacks>
