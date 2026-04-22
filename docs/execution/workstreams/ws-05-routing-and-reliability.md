# Routing and Reliability

[Back to Execution Workstreams](README.md)

Own route domain models, route selection, health signals, retries, circuit breaking, fallback, and operator diagnostics.

## Task sequence

| Task ID | Phase | Title | Depends On |
|---|---|---|---|
| RTE-001 | PI-1 | Implement route, provider target, and policy domain models | GWT-003, DB-001 |
| RTE-002 | PI-1 | Implement routing engine MVP for static selection and fallback | RTE-001, PAD-001 |
| RTE-003 | PI-2 | Implement active health probes and passive health aggregation | RTE-001, DB-002, OBS-001 |
| RTE-004 | PI-2 | Implement retry budgets, circuit breaking, and quarantine logic | RTE-002, RTE-003, OBS-001 |
| RTE-005 | PI-2 | Expose route diagnostics APIs and explanation model | RTE-002, RTE-003, CTL-001 |

## Detailed tasks

### RTE-001 — Implement route, provider target, and policy domain models

**Phase:** PI-1  
**Estimated size:** M  
**Recommended owners:** routing

**Depends on:** GWT-003, DB-001

**Primary paths to touch:**
- `crates/routing-engine`
- `crates/core-domain`
- `services/control-plane-api`

**Expected outputs:**
- route entities
- policy model
- score input structures

**Acceptance criteria:**
- route entities support weighted and ordered strategies
- validation prevents impossible configurations
- models are reusable by API and workers

**Implementation notes:**
- Every selection should produce an explanation object suitable for traces and diagnostics APIs.
- Bound retries to prevent storm amplification.
- Make manual override hooks available for operators.

### RTE-002 — Implement routing engine MVP for static selection and fallback

**Phase:** PI-1  
**Estimated size:** L  
**Recommended owners:** routing

**Depends on:** RTE-001, PAD-001

**Primary paths to touch:**
- `crates/routing-engine`
- `services/gateway-api`

**Expected outputs:**
- route evaluator
- fallback chain execution
- selection reasoning output

**Acceptance criteria:**
- engine selects eligible route deterministically
- fallback occurs on retryable error classes
- selection reason is emitted in traces

**Implementation notes:**
- Every selection should produce an explanation object suitable for traces and diagnostics APIs.
- Bound retries to prevent storm amplification.
- Make manual override hooks available for operators.

### RTE-003 — Implement active health probes and passive health aggregation

**Phase:** PI-2  
**Estimated size:** M  
**Recommended owners:** routing, sre

**Depends on:** RTE-001, DB-002, OBS-001

**Primary paths to touch:**
- `services/edge-probe`
- `services/routing-worker`
- `crates/routing-engine`

**Expected outputs:**
- probe scheduler
- health score aggregation
- route health persistence

**Acceptance criteria:**
- probe results update route health state
- cheap and billable probe modes are separated
- quarantine thresholds are configurable

**Implementation notes:**
- Every selection should produce an explanation object suitable for traces and diagnostics APIs.
- Bound retries to prevent storm amplification.
- Make manual override hooks available for operators.

### RTE-004 — Implement retry budgets, circuit breaking, and quarantine logic

**Phase:** PI-2  
**Estimated size:** L  
**Recommended owners:** routing, sre

**Depends on:** RTE-002, RTE-003, OBS-001

**Primary paths to touch:**
- `crates/routing-engine`
- `services/gateway-api`
- `services/routing-worker`

**Expected outputs:**
- retry budget policy
- circuit breaker state
- quarantine transitions

**Acceptance criteria:**
- retry loops are bounded and observable
- bad routes can be quarantined automatically
- manual override APIs exist

**Implementation notes:**
- Every selection should produce an explanation object suitable for traces and diagnostics APIs.
- Bound retries to prevent storm amplification.
- Make manual override hooks available for operators.

### RTE-005 — Expose route diagnostics APIs and explanation model

**Phase:** PI-2  
**Estimated size:** M  
**Recommended owners:** routing, control-plane

**Depends on:** RTE-002, RTE-003, CTL-001

**Primary paths to touch:**
- `services/control-plane-api`
- `crates/routing-engine`
- `schemas/openapi`

**Expected outputs:**
- diagnostics endpoints
- route explanation model
- recent failure summaries

**Acceptance criteria:**
- operators can query why a route was or was not selected
- health and score inputs are visible with redaction
- OpenAPI docs include example responses

**Implementation notes:**
- Every selection should produce an explanation object suitable for traces and diagnostics APIs.
- Bound retries to prevent storm amplification.
- Make manual override hooks available for operators.
