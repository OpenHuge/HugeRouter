# 27. Multi-Tenancy Model

[Back to Docs Index](../README.md)

### 27.0 Current Implementation Reality

The repository should currently be described this way:

- `implemented`: tenant, project, provider resource, route policy, and config snapshot concepts exist and active snapshot reads already influence the gateway
- `bootstrap-only`: control-plane auth and some admin surfaces are still oriented around seeded or bootstrap behavior
- `planned`: authoritative staged rollout, last-known-good stale diagnostics, compatibility gates, and full control-plane publish or rollback orchestration

### 27.1 Isolation Levels

The platform should support:

- logical isolation for standard multi-tenant deployments
- stronger isolation controls for enterprise tenants
- future optional dedicated single-tenant deployment model

### 27.2 Tenant Data Boundaries

All critical records must carry tenant IDs. Access paths must enforce tenant scoping centrally, not ad hoc in handlers.

### 27.3 Configuration Hierarchy

Configuration should resolve in the following order where relevant:

1. request-level override, if permitted
2. credential scope
3. project setting
4. tenant setting
5. platform default

The resolved configuration should be frozen into a request-scoped snapshot ID so that diagnostics, billing, and support can explain behavior against the exact policy state that was active at execution time.

### 27.4 Snapshot Binding Rule

Every material execution path should bind to one effective configuration snapshot before upstream execution begins.

That snapshot should govern at least:

- route policy resolution
- provider target eligibility
- budget and quota policy
- provenance and residency policy
- protocol or adapter compatibility expectations

The hot path should not reconstruct effective configuration by re-reading mutable control-plane state after the snapshot has been chosen.

---

## 28. Configuration Management

### 28.1 Configuration Sources

- static config files
- environment variables
- secret store references
- database-backed runtime configuration for control plane objects

### 28.2 Validation

All configuration should be validated at startup and runtime mutation boundaries.

### 28.3 Dynamic Reload

Some configuration classes should support dynamic reload:

- route snapshots
- pricing tables
- provider health state
- policy bundles

Highly sensitive or structural config may still require restart or rollout.

### 28.4 Staged Rollout and Safe Mutation

Runtime configuration changes should not behave like invisible global toggles.

The control plane should support:

- schema validation before persistence
- route simulation against pending changes
- staged rollout by tenant, region, or percentage
- fast rollback to a prior config snapshot
- emergency kill switches for tenants, credentials, or provider resources

This is especially important for routing, pricing, provenance policy, and redaction settings, where a bad mutation can create immediate spend, availability, or compliance incidents.

### 28.5 Control Plane and Data Plane Snapshot Contract

Configuration publication should follow an explicit CP/DP contract:

1. control plane validates schema and cross-resource references
2. control plane computes an activation candidate snapshot
3. compatibility checks run against adapter manifests and runtime version contracts
4. data plane adopts the new snapshot only after validation succeeds
5. if adoption fails, the data plane continues using the last-known-good snapshot

The data plane should expose whether it is serving:

- current snapshot
- stale but last-known-good snapshot
- no valid snapshot available

### 28.6 Compatibility Gate

Snapshot activation should check compatibility across at least:

- config snapshot schema version
- current route-policy and provider-resource contract versions, with future extension to typed route-resource versions once those contracts exist
- adapter manifest compatibility range
- runtime binary compatibility version

An activation that fails compatibility must not partially mutate live routing behavior.

### 28.7 Rollback and Stale-Serving Rules

Operational rules:

- CP unavailability must not require ordinary request execution to round-trip to mutable control-plane state
- DP instances may continue serving from the last-known-good compatible snapshot for a bounded stale window
- stale-serving status must be visible to diagnostics and operations tooling
- emergency rollback should reactivate the most recent known-compatible snapshot, not reconstruct state ad hoc

---
