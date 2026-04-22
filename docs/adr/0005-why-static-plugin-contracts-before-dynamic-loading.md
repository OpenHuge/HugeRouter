# ADR 0005: Why Static Plugin Contracts Before Dynamic Loading

[Back to ADR Index](README.md)

## Status

Accepted

## Context

The platform is intended to be extensible through adapters, protocol handlers, policy stages, and future plugin-style boundaries. A tempting path would be to start with dynamic plugin loading immediately, but that would force early decisions around ABI stability, sandboxing, dependency isolation, rollout safety, and operational debugging before the core contracts have matured.

We need an architecture that is genuinely pluggable without making the first implementation unnecessarily heavy or fragile.

## Decision

Adopt **static plugin contracts plus config-driven registration** as the initial extensibility model.

That means:

- extension points are defined through stable traits and manifests
- implementations are compiled into the binary
- services register implementations during bootstrap through explicit composition roots
- runtime selection happens through registries, capability matching, and configuration

Dynamic loading remains a possible future step, but it is not part of the initial architecture baseline.

## Alternatives Considered

### Dynamic libraries from the start

This would maximize theoretical flexibility, but it introduces ABI, packaging, isolation, upgrade, and debugging complexity before we have validated the extension contracts themselves.

### WASM plugin runtime from the start

WASM is attractive for sandboxing, but it still adds a substantial runtime model, capability boundary, and host-call design problem. It is a better second-phase evolution after the contracts are proven.

### No plugin model, only hard-coded modules

This is simpler at first, but it would make future provider, protocol, and policy growth expensive and would encourage service-level branching instead of proper composition.

## Consequences

Positive:

- cleaner service assembly through explicit composition roots
- extension points can be tested and versioned early
- lower implementation and operational complexity in the first release
- easier future transition to dynamic loading if contracts remain stable

Tradeoffs:

- adding a new module still requires a rebuild and deploy
- external third-party plugins are deferred
- discipline is required so registries and manifests remain authoritative

## Follow-up Actions

- define shared manifest and capability contracts in `plugin-sdk`
- add `plugin-registry` and `runtime-composition` crates
- make provider and protocol implementations register through explicit bootstrap code
- expose discoverable manifest metadata through diagnostics and control plane APIs

