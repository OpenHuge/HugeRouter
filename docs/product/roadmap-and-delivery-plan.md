# 32. Migration and Rollout Plan

[Back to Docs Index](../README.md)

## 32.0 Product Direction Update: ku0 Trust Layer

As of the 2026-04-26 product analysis, the first commercial direction for this
repository is **ku0 Trust Layer**, an AI-resource quality inspection, supplier
risk, procurement evidence, and monitored gateway platform.

The gateway remains essential infrastructure, but it should be presented as the
evidence-producing substrate for supplier quality and trusted procurement rather
than as the only product. For the detailed product definition and first public
launch plan, see [ku0 Trust Layer Product Plan V1.0](ku0-trust-layer-v1.md).

The planning hierarchy is now:

1. ku0 Probe and Reports prove resource quality before purchase.
2. ku0 Monitor preserves long-running evidence after adoption.
3. ku0 Verified lets suppliers buy inspection without buying conclusions.
4. ku0 Trusted Gateway routes only through inspected resources and feeds real
   production quality data back into supplier trust scores.

### 32.1 Phase 0: Foundation

Deliver:

- monorepo scaffolding
- Rust workspace
- TanStack Start app shell
- bilingual console shell baseline for Simplified Chinese and English
- PostgreSQL/Redis/NATS local stack
- auth skeleton
- trace pipeline skeleton
- code-first review of similar project source or official technical material
- a strengths-to-spec mapping that identifies which borrowed behaviors are implemented, bootstrap-only, or planned

### 32.2 Phase 1: Minimal Viable Gateway

Deliver:

- OpenAI-compatible northbound ingress
- at least one native upstream adapter
- API key auth
- basic route policy
- usage capture
- simple admin console

ku0 Trust Layer interpretation:

- treat this phase as the internal Trusted Gateway substrate
- ensure route receipts, usage events, and provider provenance are suitable for
  later inspection reports
- do not market this phase as a generic relay marketplace

### 32.3 Phase 2: Multi-Protocol Gateway & Semantic Edge

Deliver:

- Anthropic and Gemini northbound support
- richer IR with MCP (Model Context Protocol) via Streamable HTTP
- Semantic caching layer with pgvector for repeated agent prompts
- multiple upstream adapters
- provider-resource management for OpenAI-compatible relay and gateway endpoints
- compatibility profiles for common upstream relay families such as One API, New API, Sub2API, LiteLLM, and generic OpenAI-compatible brokers
- retry/fallback logic
- route diagnostics UI

ku0 Trust Layer interpretation:

- use multi-protocol support to broaden inspection coverage
- keep model and protocol compatibility results evidence-backed and scoped
- distinguish live checks from simulated checks in both API and console surfaces

### 32.4 Phase 3: Ledger and Billing Maturity

Deliver:

- immutable ledger
- pricing engine
- balance projections
- billing dashboard
- WeChat Pay as the first supported payment collection method
- audit improvements
- replay capsule and supportability maturity

ku0 Trust Layer interpretation:

- promote billing and metering work into Billing Transparency reports
- calculate real effective cost, failed-request cost, retry cost, and
  traceable billing evidence where data is available

### 32.5 Phase 4: Enterprise, Realtime, and Ecosystem

Deliver:

- Realtime WebRTC proxy targeting OpenAI Realtime API GA
- MCP server orchestration with stateless Streamable HTTP transport
- A2A (Agent-to-Agent) protocol support with Agent Card discovery and task lifecycle management
- A2A Hub-and-Spoke gateway governance model
- Non-Human Identity (NHI) management for agent credentials
- SSO and advanced RBAC including agent roles
- data residency policies
- plugin/adapters expansion
- gateway-of-gateways support
- hardened compatibility profiles for multiple third-party relay families such as One API, New API, Sub2API, LiteLLM, and LMRouter-style deployments
- advanced observability and analytics with agentic session tracing
- optional orchestration-aware integrations with external agent runtimes and memory systems where product demand justifies them

ku0 Trust Layer interpretation:

- enterprise features should center on supplier whitelist, procurement evidence,
  continuous monitoring, incident evidence packs, and private inspection suites
- broad protocol expansion is valuable only when it strengthens procurement and
  supplier trust decisions

---

## 32.6 First Public ku0 Launch Plan

This is the concrete path from the current implementation to the first public
version of ku0 Trust Layer.

### Stage 0: Documentation And Product Repositioning

Deliver:

- ku0 Trust Layer product plan
- updated docs index and roadmap
- clear statement that card-secret/account resale is not the core product
- existing merchant relay evaluation labeled as bootstrap supplier evidence

### Stage 1: Current Baseline Hardening

Deliver:

- end-to-end verification of control plane, gateway, route receipts, pricing,
  billing, merchant evaluation, replay capsule, and edge-probe flows
- stronger tests for simulated relay evaluation and replay access
- UI/API labels that make `simulated` impossible to confuse with live evidence
- demo data for supplier profiles and inspection examples

### Stage 2: Live ku0 Probe MVP

Deliver:

- live OpenAI-compatible probe runner
- base URL, API key, configured model, non-streaming, streaming, first-token
  latency, error-code, and token-usage checks
- redacted JSON report output
- stored result summaries and response hashes
- console action for running a live endpoint inspection

### Stage 3: Supplier Profiles And Shareable Reports

Deliver:

- supplier profile model
- public-safe report slug
- HTML report page
- status summary with latest check time, availability window, latency, error
  rate, streaming stability, and risk notes
- supplier response and moderation fields

### Stage 4: Reports And Certification Beta

Deliver:

- manual-assisted paid report workflow
- single-supplier, comparison, billing anomaly, model-consistency, and launch
  readiness report types
- initial certification states: Basic Checked, Billing Transparent, Risk Watch,
  and Not Recommended
- certification expiry and re-check triggers
- evidence labels for supplier-paid, ku0-self-tested, customer-submitted,
  simulated, and live results

### Stage 5: Public Beta Launch

Deliver:

- public ku0 Trust Layer homepage copy
- curated supplier directory
- public report examples
- supplier response and re-check support flow
- launch metrics dashboard

Launch exit criteria:

- at least 20 supplier profiles
- at least 5 publishable live probe reports
- at least 2 report or certification purchase-intent workflows completed
- all public conclusions display sample window, sample count, evidence source,
  and detection scope

---

## 39. Future Extension Points

### 39.1 Plugin Runtime

A future plugin system may support:

- custom adapters
- custom metering rules
- custom policy hooks
- request/response enrichers

WASM is a strong candidate for sandboxed extension execution.

### 39.2 Enterprise Features

Future enterprise extensions may include:

- custom tenant branding
- BYOK/BYOC provider resource models
- private regional deployments
- dedicated route clusters
- advanced compliance exports

### 39.3 AI-Assisted Operations

Future internal assistants may help with:

- route recommendation
- incident diagnosis
- cost anomaly explanation
- policy generation suggestions

---

## 40. Risks and Mitigations

### 40.1 Risk: Over-Abstracting Too Early

Mitigation:

- start with a minimal but strong IR
- preserve provider-specific metadata fields
- only generalize where real use cases exist
- validate new abstractions against concrete source-backed behavior from similar systems before locking them into the spec
- prefer one copied operationally-proven behavior over three speculative generic extension points

### 40.2 Risk: Monorepo Complexity

Mitigation:

- strict package boundaries
- ownership rules
- selective CI execution
- architecture decision records

### 40.3 Risk: Billing Errors

Mitigation:

- immutable ledger
- idempotency keys
- replayable events
- strong auditability
- projection repair jobs

### 40.4 Risk: Route Instability

Mitigation:

- quarantine state
- route simulation
- bounded retries
- explicit route policies
- health score smoothing

### 40.5 Risk: Security Drift

Mitigation:

- private admin plane by default
- secret store integration
- mandatory audit trails
- least privilege defaults

### 40.6 Risk: A2A Infinite Loops

Mitigation:

- A2A request hop limits
- agent identity validation
- circuit breaking on token bursts

### 40.7 Risk: Semantic Cache Invalidation

Mitigation:

- TTL-based invalidation
- content-hash keys
- developer-controlled purge API

---

## 41. Open Questions

1. Should the first release support both OpenAI Chat Completions and Responses, or only one northbound protocol initially?
2. Should route policy evaluation be purely in-process, or should advanced policy evaluation delegate to OPA from the beginning?
3. Should analytics use PostgreSQL initially and defer ClickHouse, or be included from the start?
4. Should the first production deployment target Kubernetes only, or also support a simpler Docker Compose self-hosted mode as a first-class deliverable?
5. Should the plugin/adapters roadmap be public and stable early, or remain internal until the core abstractions settle?
6. How deeply should the AI Gateway introspect MCP (Model Context Protocol) tool calls versus passing them transparently?
7. Should A2A Agent Card discovery and validation be implemented as a gateway-native feature or delegated to an external service registry?
8. Should semantic caching use pgvector within PostgreSQL or a dedicated vector store from the start?
9. How should the gateway handle A2A delegation chain depth limits to prevent cascading agent loops while still supporting legitimate multi-agent workflows?
10. Which memory-bearing operations, if any, should be classified specially in policy and diagnostics before the platform ever owns a memory subsystem?

## 41.1 Recommended Answers For Development Kickoff

To avoid blocking implementation, use these defaults unless a later ADR changes them:

- model the gateway around Responses-era semantics, even if Chat Completions compatibility ships first
- keep route policy evaluation in-process for the first release
- defer ClickHouse
- make Docker Compose the first self-hosted and developer baseline
- keep plugin contracts stable early, but defer third-party dynamic plugin loading
- treat MCP as a first-class protocol family and inspect it enough for auth, policy, routing, and observability boundaries
- treat A2A as a production-ready protocol and implement Agent Card discovery and task lifecycle from the first A2A milestone
- start semantic caching with pgvector and defer dedicated vector store until scale requires it
- implement NHI management as an extension of the existing credential model rather than a separate subsystem
- treat external orchestration and memory systems as governed integrations first, not as runtime responsibilities the gateway must absorb immediately

---

## 42. Recommended Initial Delivery Slice

If the team wants the smallest strategically correct first milestone, ship the following:

- Rust monorepo workspace
- TanStack Start console shell
- `gateway-api` and `control-plane-api`
- OpenAI-compatible northbound ingress
- OpenAI native upstream adapter
- API key auth
- PostgreSQL + Redis + NATS local stack
- a reference-study output tied to routing, config snapshots, admission, and observability
- basic route policy engine
- immutable usage events and simple ledger entries
- basic usage dashboard
- basic route diagnostics
- OpenTelemetry tracing

This is enough to prove the architecture without prematurely committing to every advanced feature.

---

## 43. Conclusion

This monorepo specification now supports ku0 Trust Layer as a **future-facing AI-resource trust and procurement evidence platform** rather than a conventional transit panel or generic gateway product.

The architecture is intentionally designed around:

- Rust for backend correctness, performance, and strong domain modeling
- TanStack Start for a modern, typed frontend console
- a monorepo for shared contracts and coordinated delivery
- strict separation between control plane and data plane
- immutable metering and ledger design
- route intelligence as a first-class system capability
- enterprise-grade security and observability foundations

If implemented as specified, this platform can evolve from an initial multi-provider gateway substrate into a durable trust layer for AI-resource procurement, supplier monitoring, commercial reports, certification, enterprise governance, and trusted production routing.

---

## Appendix C: Example Milestone Backlog Categories

- repository scaffolding
- protocol ingestion
- auth and identity
- route engine
- provider adapters
- usage metering
- ledger and pricing
- frontend foundations
- observability
- security hardening
- deployment and operations
- documentation and ADRs
