# Parallel Workstreams

[Back to Execution Index](README.md)

The roadmap is intentionally arranged as **parallel workstreams**. Agents should prefer taking tasks from different streams when the touched paths do not overlap.

## Workstream summary

| Workstream | Earliest Start | Primary Unlocks / Dependencies | Task Count |
|---|---|---|---|
| Foundation and Monorepo | PI-0 | none | 6 |
| Data Storage and Events | PI-0 | FND-001, FND-003 | 2 |
| Gateway and Protocol Ingress | PI-1 | FND-001, FND-003, GWT-003, PAD-001, RTE-002 | 8 |
| Provider Adapters | PI-1 | GWT-003 | 5 |
| Routing and Reliability | PI-1 | GWT-003, DB-001, PAD-001 | 5 |
| Control Plane and Console | PI-0 | FND-002, CTL-001, DB-001, SEC-001 | 7 |
| Metering, Ledger, and Billing | PI-1 | GWT-003, DB-002 | 5 |
| Security, Identity, and Compliance | PI-0 | FND-004, CTL-001 | 4 |
| Observability, SRE, and Runtime | PI-0 | FND-003 | 4 |
| QA, Release, and Documentation | PI-0 | FND-005, FND-006 | 4 |

## Parallelization rules

1. **Lock interfaces early, not implementations.**  
   Create IR, adapter traits, OpenAPI documents, shared config, and repository contracts before multiple agents build on top of them.

2. **Prefer horizontal slices during PI-0 and PI-1.**  
   While one agent builds the Rust workspace, another can scaffold the TanStack Start shell, while a third sets up the local infra stack and CI.

3. **Avoid concurrent edits to the same boundary crates.**  
   `crates/core-domain`, `crates/protocol-ir`, `crates/provider-traits`, and `crates/storage` are high-contention areas. Assign explicit ownership windows.

4. **Control plane and gateway can progress separately after shared primitives exist.**  
   Once schema generation, auth model, storage primitives, and service skeletons exist, frontend and gateway work can scale independently.

5. **Billing and analytics must not block the gateway MVP.**  
   Only the usage event contract is on the hot path. Projection logic, pricing, and billing dashboards can proceed later in parallel.

## Recommended concurrent lanes

### Lane A — Platform foundation
- FND-001
- FND-003
- FND-005
- OBS-001

### Lane B — Frontend foundation
- FND-002
- FND-006
- CTL-004

### Lane C — Data and control plane foundation
- CTL-001
- DB-001
- DB-002
- SEC-001

### Lane D — Gateway kernel
- GWT-001
- GWT-003
- PAD-001
- RTE-001

After those lanes land, the roadmap fans out into:
- gateway MVP
- provider adapters
- routing diagnostics
- UI management flows
- billing and ledger
- SSO and enterprise controls
