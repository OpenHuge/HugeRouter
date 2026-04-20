# Open Source Reference Patterns

[Back to Docs Index](../README.md)

This document records the architecture patterns we are intentionally borrowing from leading open source products, and how those patterns should influence implementation.

The goal is not to clone any single product. The goal is to avoid inventing first-version architecture in areas where mature projects have already proven better defaults.

## 1. Selection Criteria

Reference products were selected because they are strong in one or more of these dimensions:

- control plane and data plane separation
- extension and plugin architecture
- typed configuration and versioning
- observability contracts
- multi-tenant gateway operations
- admin console composition in large monorepos

The current reference set is:

- Backstage
- Envoy
- Envoy AI Gateway
- Kong Gateway
- OpenTelemetry
- LiteLLM

## 1.1 Community Pressure Signals

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

## 8. What We Adopt Immediately

The following are now considered implementation defaults:

- package and plugin boundaries are explicit and app-first
- request-path ordering is part of the architecture contract
- manifests and extension configs are typed and versioned
- CP/DP separation is real, not merely conceptual
- telemetry naming is governed centrally
- gateway keys, upstream secrets, budgets, and routing strategies are first-class domain concepts

## 9. What We Deliberately Do Not Copy

We are not copying these parts directly:

- Backstage's full plugin runtime and developer portal mental model
- Envoy's full xDS control plane complexity
- Kong's exact deployment topology or operational product model
- LiteLLM's Python implementation model or OpenAI-format-first worldview as the long-term platform boundary

These projects are reference patterns, not product templates.

## 10. Development Implication

Because these reference patterns are now captured explicitly, implementation should assume:

- shared packages and crates are contracts, not convenience folders
- routing and auth order must be testable
- CP/DP compatibility, config snapshotting, and telemetry semantics are build-time concerns, not cleanup tasks for later
- feature work that breaks these patterns should be treated as an architecture change, not a small refactor
