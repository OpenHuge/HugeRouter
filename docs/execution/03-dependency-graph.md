# Dependency Graph

[Back to Execution Index](README.md)

## Critical path

The true critical path is shorter than the full task list. The program should optimize to land the following chains early:

- FND-001 → FND-004 → CTL-001 → SEC-001 → CTL-002 / CTL-003 → GWT-002
- FND-001 → GWT-003 → PAD-001 → PAD-002 → RTE-002 → GWT-004 → GWT-005
- FND-003 → DB-001 / DB-002 → MET-001 → MET-002 → MET-003 → MET-004 → CTL-007
- OBS-001 → OBS-002 → OBS-003 / RTE-003 → RTE-004 → GWT-008 / PAD-005

## Dependency guidance

### Hard dependencies

Use a hard dependency when:

- a task cannot compile without another task's artifacts
- a task needs a stable interface, schema, or migration before implementation
- a task depends on seeded data or a running integration environment

### Soft dependencies

Use a soft dependency when:

- another task improves completeness but is not required for the first implementation
- API/UI work can proceed against mocks or generated schemas
- the feature can ship in a degraded or hidden state

## High-contention areas

These paths should have explicit temporary ownership when active work is underway:

- `crates/core-domain`
- `crates/protocol-ir`
- `crates/provider-traits`
- `crates/storage`
- `schemas/openapi`
- `packages/ts-api-client`

## Recommended merge order

1. foundational workspace + local infra + telemetry
2. storage + control plane skeleton + RBAC
3. protocol IR + provider traits + routing models
4. first adapter + routing MVP + gateway ingress
5. schema generation + TanStack shell + CRUD UI
6. usage events + ledger + projections
7. diagnostics, health, retries, audit, alerts
8. SSO, billing UI, performance, release maturity

## Practical rule for agents

If two tasks touch the same boundary crate and there is no clear interface freeze, do **not** run them concurrently. Merge the interface owner first, then let implementation tasks fan out.
