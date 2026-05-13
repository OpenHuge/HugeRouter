# PI-3 — Billing and Enterprise Hardening

[Back to Task Catalog Index](README.md)

Ledger, billing, SSO, analytics, and operational hardening tasks.

## Recommended execution order

- **CTL-007** — Build usage, budget, and billing dashboards _(depends on: MET-004, MET-005, CTL-004)_
- **MET-003** — Implement pricing engine with token, cached-token, and multimodal units _(depends on: MET-002, CTL-003)_
- **MET-004** — Build balance, budget, and projection read models _(depends on: MET-002, MET-003)_
- **MET-005** — Implement billing and usage analytics endpoints _(depends on: MET-004, CTL-001)_
- **OBS-004** — Create load, soak, and failure-injection test suite _(depends on: GWT-005, RTE-004, MET-002)_
- **SEC-002** — Implement OIDC/SSO login flow for console users _(depends on: CTL-004, SEC-001)_

## Detailed tasks

### CTL-007 — Build usage, budget, and billing dashboards

**Stream:** Control Plane and Console  
**Owners:** frontend-console  
**Depends on:** MET-004, MET-005, CTL-004

**Paths:**

- `apps/console-web/src/routes/usage`
- `apps/console-web/src/routes/billing`

**Outputs:**

- usage charts
- budget threshold views
- balance and invoice projections
- payment initiation and status surfaces for the first billing collection flow

**Acceptance criteria:**

- dashboards match projection APIs
- time-range filters and tenant scopes work
- large tables are paginated and exportable
- the first payment flow exposed in the console supports WeChat Pay only and states that scope clearly

### MET-003 — Implement pricing engine with token, cached-token, and multimodal units

**Stream:** Metering, Ledger, and Billing  
**Owners:** billing-platform  
**Depends on:** MET-002, CTL-003

**Paths:**

- `crates/metering`
- `crates/ledger-models`
- `services/control-plane-api`

**Outputs:**

- pricing rules engine
- price table model
- cost calculation library

**Acceptance criteria:**

- pricing can model input/output/cache/image/audio units
- pricing rules are versioned and auditable
- simulation endpoint exists for test scenarios

### MET-004 — Build balance, budget, and projection read models

**Stream:** Metering, Ledger, and Billing  
**Owners:** billing-platform  
**Depends on:** MET-002, MET-003

**Paths:**

- `services/ledger-worker`
- `crates/storage`
- `services/control-plane-api`

**Outputs:**

- projection jobs
- budget threshold evaluation
- usage summary APIs

**Acceptance criteria:**

- projection lag is measurable
- budget threshold events are emitted once per threshold crossing
- read models can be repaired without ledger mutation

### MET-005 — Implement billing and usage analytics endpoints

**Stream:** Metering, Ledger, and Billing  
**Owners:** billing-platform, control-plane  
**Depends on:** MET-004, CTL-001

**Paths:**

- `services/control-plane-api`
- `schemas/openapi`

**Outputs:**

- aggregated usage endpoints
- balance endpoints
- billing export endpoints
- payment record and status endpoints for the initial collection flow

**Acceptance criteria:**

- APIs support tenant/project/time filtering
- responses are paginated for large datasets
- exports are asynchronous where needed
- the first payment collection path is modeled for WeChat Pay without leaking provider-specific semantics into ledger records

### OBS-004 — Create load, soak, and failure-injection test suite

**Stream:** Observability, SRE, and Runtime  
**Owners:** sre, qa  
**Depends on:** GWT-005, RTE-004, MET-002

**Paths:**

- `crates/testing-kit`
- `infra/scripts`
- `docs/runbooks`

**Outputs:**

- performance test scenarios
- failure injection harness
- capacity baseline report

**Acceptance criteria:**

- test suite covers concurrency, retry storms, and degraded dependencies
- results are versioned and comparable
- capacity thresholds are documented

### SEC-002 — Implement OIDC/SSO login flow for console users

**Stream:** Security, Identity, and Compliance  
**Owners:** security, frontend-console  
**Depends on:** CTL-004, SEC-001

**Paths:**

- `apps/console-web`
- `services/control-plane-api`
- `crates/authn-authz`

**Outputs:**

- OIDC login flow
- session handling
- group-to-role mapping

**Acceptance criteria:**

- interactive login works against a reference IdP
- logout and session expiry are handled cleanly
- role mapping is configurable
