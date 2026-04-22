# Metering, Ledger, and Billing

[Back to Execution Workstreams](README.md)

Capture usage on the hot path, write immutable ledger records, compute projections, and expose billing analytics.

## Task sequence

| Task ID | Phase | Title                                                                   | Depends On              |
| ------- | ----- | ----------------------------------------------------------------------- | ----------------------- |
| MET-001 | PI-1  | Define usage event schema and hot-path usage extraction hooks           | GWT-003, DB-002         |
| MET-002 | PI-2  | Build ledger worker for immutable event ingestion and deduplication     | MET-001, DB-001, DB-002 |
| MET-003 | PI-3  | Implement pricing engine with token, cached-token, and multimodal units | MET-002, CTL-003        |
| MET-004 | PI-3  | Build balance, budget, and projection read models                       | MET-002, MET-003        |
| MET-005 | PI-3  | Implement billing and usage analytics endpoints                         | MET-004, CTL-001        |

## Detailed tasks

### MET-001 — Define usage event schema and hot-path usage extraction hooks

**Phase:** PI-1  
**Estimated size:** M  
**Recommended owners:** billing-platform

**Depends on:** GWT-003, DB-002

**Primary paths to touch:**

- `crates/metering`
- `crates/ledger-models`
- `services/gateway-api`

**Expected outputs:**

- usage event schema
- gateway emission hooks
- idempotency key format

**Acceptance criteria:**

- usage events can be emitted for success and failure paths
- schema version is explicit
- events include tenant/project/route correlation IDs

**Implementation notes:**

- The gateway hot path should only emit usage events, never compute full balances.
- Ledger persistence must be append-only and replayable.
- Projections are disposable and repairable; ledger entries are not.

### MET-002 — Build ledger worker for immutable event ingestion and deduplication

**Phase:** PI-2  
**Estimated size:** L  
**Recommended owners:** billing-platform

**Depends on:** MET-001, DB-001, DB-002

**Primary paths to touch:**

- `services/ledger-worker`
- `crates/ledger-models`
- `crates/storage`

**Expected outputs:**

- immutable ledger persistence
- deduplication logic
- repair/replay hooks

**Acceptance criteria:**

- duplicate usage events are ignored safely
- ledger writes are append-only
- replay against a snapshot produces deterministic results

**Implementation notes:**

- The gateway hot path should only emit usage events, never compute full balances.
- Ledger persistence must be append-only and replayable.
- Projections are disposable and repairable; ledger entries are not.

### MET-003 — Implement pricing engine with token, cached-token, and multimodal units

**Phase:** PI-3  
**Estimated size:** L  
**Recommended owners:** billing-platform

**Depends on:** MET-002, CTL-003

**Primary paths to touch:**

- `crates/metering`
- `crates/ledger-models`
- `services/control-plane-api`

**Expected outputs:**

- pricing rules engine
- price table model
- cost calculation library

**Acceptance criteria:**

- pricing can model input/output/cache/image/audio units
- pricing rules are versioned and auditable
- simulation endpoint exists for test scenarios

**Implementation notes:**

- The gateway hot path should only emit usage events, never compute full balances.
- Ledger persistence must be append-only and replayable.
- Projections are disposable and repairable; ledger entries are not.

### MET-004 — Build balance, budget, and projection read models

**Phase:** PI-3  
**Estimated size:** M  
**Recommended owners:** billing-platform

**Depends on:** MET-002, MET-003

**Primary paths to touch:**

- `services/ledger-worker`
- `crates/storage`
- `services/control-plane-api`

**Expected outputs:**

- projection jobs
- budget threshold evaluation
- usage summary APIs

**Acceptance criteria:**

- projection lag is measurable
- budget threshold events are emitted once per threshold crossing
- read models can be repaired without ledger mutation

**Implementation notes:**

- The gateway hot path should only emit usage events, never compute full balances.
- Ledger persistence must be append-only and replayable.
- Projections are disposable and repairable; ledger entries are not.

### MET-005 — Implement billing and usage analytics endpoints

**Phase:** PI-3  
**Estimated size:** M  
**Recommended owners:** billing-platform, control-plane

**Depends on:** MET-004, CTL-001

**Primary paths to touch:**

- `services/control-plane-api`
- `schemas/openapi`

**Expected outputs:**

- aggregated usage endpoints
- balance endpoints
- billing export endpoints
- payment record and status endpoints for the initial collection flow

**Acceptance criteria:**

- APIs support tenant/project/time filtering
- responses are paginated for large datasets
- exports are asynchronous where needed
- the first payment collection path is modeled for WeChat Pay without hardcoding provider-specific semantics into ledger records

**Implementation notes:**

- The gateway hot path should only emit usage events, never compute full balances.
- Ledger persistence must be append-only and replayable.
- Projections are disposable and repairable; ledger entries are not.
- Model payment attempts and confirmations separately from funding ledger entries so more payment methods can be added later without changing billing invariants.
