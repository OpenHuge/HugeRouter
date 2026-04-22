# 13. Extensibility and Composition

[Back to Docs Index](../README.md)

The system should be architected around **ports, registries, manifests, and composition roots** so that new protocols, providers, policies, and side effects can be added without rewriting the request path.

### 13.1 Design Goal

We want the platform to be:

- **pluggable**: new modules can be added behind stable contracts
- **composable**: services assemble behavior from reusable stages instead of monolithic handler code
- **observable**: every extension point can be inspected, validated, and traced
- **safe to evolve**: new modules do not force broad rewrites or hidden coupling

### 13.2 Architectural Pattern

The recommended baseline is:

1. keep domain and IR types stable
2. define extension contracts in shared SDK crates
3. register implementations in a runtime registry
4. assemble each service through an explicit composition root
5. compose request processing as middleware and stage pipelines

This gives us a plugin-ready architecture without requiring dynamic loading on day one.

### 13.3 Compile-Time Plugins First

For the initial implementation, pluggable means:

- modules are compiled into the binary
- service bootstrap registers the available implementations
- control plane and diagnostics can inspect manifest metadata
- runtime behavior is selected by configuration and capability matching

This is intentionally different from shipping a dynamic plugin runtime immediately. Dynamic loading can come later after contracts, versioning, and sandbox requirements are proven.

### 13.4 Primary Extension Points

The first-class extension points should be:

- protocol parsers and serializers
- provider adapters
- A2A agent adapters and Agent Card resolvers
- semantic cache strategies
- policy stages and evaluators
- metering extractors and sinks
- notification transports
- audit exporters
- diagnostics enrichers

Guardrail from recent agent frameworks:

- extension points that model durable task orchestration, human approval, or memory should earn their existence through a concrete workflow need, not through speculative framework symmetry
- the gateway may integrate with orchestration or memory systems without having to internalize their entire runtime model

Each extension point should define:

- a stable trait or interface
- a manifest structure
- config validation rules
- capability descriptors
- testing and conformance expectations

### 13.5 Composition Root Responsibilities

Each service should have one composition root that is responsible for:

- loading config
- building infrastructure clients
- registering protocol handlers and adapters
- constructing middleware chains
- wiring policy and routing engines
- exposing discoverable manifests to diagnostics and admin APIs

Business logic crates should not perform hidden singleton registration or implicit global state mutation.

### 13.6 Registry Responsibilities

Registries should be used wherever a service selects among pluggable modules.

Registry responsibilities:

- register module factories
- validate duplicate IDs and conflicting manifests
- resolve modules by capability, version, and runtime constraints
- list manifests for diagnostics and control plane discovery
- support feature-flag or environment-based enablement

### 13.7 Middleware and Pipeline Composition

Cross-cutting behavior should be modeled as composable stages.

Recommended request-path composition:

1. auth and tenant resolution
2. protocol parsing
3. IR validation
4. policy evaluation
5. route selection
6. adapter resolution
7. retry, timeout, and resilience middleware
8. adapter execution
9. response normalization
10. usage extraction and event publication
11. telemetry, diagnostics, and audit hooks

This makes behavior rearrangeable, testable, and replaceable without cloning service handlers.

Additional composition rule:

- request-path middleware should remain distinct from long-running task orchestration machinery
- if a future feature requires resumable state machines, model that as an adjacent orchestration surface with explicit handoff points rather than burying it inside ordinary HTTP middleware

### 13.8 Dependency Rules

- domain crates must not depend on service crates
- contracts and SDK crates must not depend on infrastructure implementations
- adapter crates implement contracts but do not own service wiring
- service crates own assembly, configuration, and runtime orchestration
- control plane APIs should consume manifests and capability metadata instead of hard-coded provider knowledge

### 13.9 Versioning Rules

Every pluggable contract should carry an explicit versioning strategy:

- manifest version
- config schema version
- capability descriptor version
- compatibility guarantees for conformance tests

Breaking a plugin contract should be treated like breaking a public interface, even if the first implementation is internal-only.

### 13.10 What Not To Do

Avoid these anti-patterns:

- giant `match` statements that encode provider behavior across unrelated crates
- routing logic that knows provider implementation details directly
- hidden global registries initialized by side effects
- plugin-specific metadata stored only as untyped blobs without manifest contracts
- dynamic plugin loading before contract stability, observability, and sandboxing are defined

### 13.11 Immediate Development Guidance

Before building many adapters or protocols, implement these foundations first:

- `plugin-sdk`
- `plugin-registry`
- `runtime-composition`
- manifest and capability descriptors
- adapter and protocol conformance tests

This is the smallest architecture slice that makes later growth truly composable instead of just modular on paper.

---
