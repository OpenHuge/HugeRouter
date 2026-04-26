# 14. Metering and Ledger

[Back to Docs Index](../README.md)

### 14.0 Current Implementation Reality

The repository should currently be described this way:

- `implemented`: domain contracts and examples for usage, receipts, and ledger-oriented concepts are stronger than the running persistence path
- `bootstrap-only`: parts of the control-plane and schema surface imply richer usage and receipt handling than the gateway currently persists
- `planned`: durable usage-event persistence, durable route-receipt persistence, projection repair jobs, and runtime budget reservation or reconciliation

### 14.1 Design Principle

Metering records what happened. Billing interprets what it means commercially.

### 14.1.1 Canonical Object Boundaries

The platform should keep these objects separate:

- `UsageEvent`: immutable fact about measured consumption or avoided consumption
- `BudgetState`: mutable guardrail state used for pre-admission and post-execution enforcement
- `PricingCatalog`: reference data used to translate usage into cost or billable price
- `LedgerEntry`: immutable financial interpretation of usage, adjustment, reserve, or release
- `Projection`: mutable read model derived from ledger and usage events

These objects may reference one another, but they should not collapse into one record type.

### 14.2 Usage Dimensions

The system must support multiple usage dimensions, including:

- input tokens
- output tokens
- cached input tokens
- image generation tier
- image pixels or size bands
- audio seconds
- video seconds
- realtime session minutes
- tool call count
- file storage bytes
- A2A task delegation count
- A2A task compute duration
- semantic cache hit savings (avoided upstream tokens)
- vector operation units if added in the future

### 14.3 Usage Event Flow

1. request starts
2. estimated usage and cost envelope calculated for admission control
3. reserve or pre-authorization recorded when policy requires it
4. route selected
5. upstream emits partial or final response (or continuous frames in a `ws` stream)
6. adapter extracts usage fields
7. **Progressive Metering:** For long-lived WebSocket/Realtime sessions, adapters emit partial normalized usage events periodically (e.g., every minute or per tool-call) rather than waiting for connection termination. This is critical for interrupting runaway agent loops before they cause a "Denial of Wallet".
8. normalized usage event emitted to message bus
9. ledger worker persists immutable usage event
10. reserve is reconciled into final debit or release entries
11. projection tables update balances, cost summaries, and analytics, triggering circuit breakers if quotas are exceeded.

### 14.3.1 Budget Reservation Lifecycle

Budget and spend control should use a consistent lifecycle:

1. pre-admission estimate computes expected marginal cost or usage envelope
2. reservation or deny decision occurs before upstream execution when policy requires it
3. in-flight adjustments may extend or clamp the reserve for long-lived sessions
4. terminal reconciliation converts reserve to final debit plus release of unused hold

Target semantics:

- pre-admission reservation is part of admission control, not post-facto analytics
- long-lived sessions may require incremental reserve top-ups or forced termination
- every reservation decision should map to a stable outcome vocabulary such as `not_required`, `reserved`, `partially_reserved`, `denied`, or `reconciled`

### 14.4 Ledger Entry Types

- debit usage cost
- credit manual adjustment
- credit promotional balance
- debit refund reversal
- debit minimum fee
- reserve pre-authorization
- release unused reserve

### 14.5 Projection Models

Derived projections include:

- current balance by tenant/project
- spend by day/week/month
- spend by model alias
- spend by upstream provider
- gross margin by route
- cached token savings estimate
- semantic cache hit rate and savings by tenant/project
- A2A task volume and cost by delegating agent
- anomaly score

### 14.6 Idempotency

Ledger ingestion must be idempotent. Duplicate usage events must not double-charge.

### 14.7 Budget and Quota Guardrail Semantics

The platform should support both soft and hard economic guardrails:

- soft warning thresholds that annotate diagnostics and customer dashboards
- hard stop thresholds that reject or terminate traffic before additional spend accrues
- tenant or project budgets with explicit reset windows
- provider-resource ceilings used to protect shared upstream capacity
- session kill-switches for runaway long-lived streams or tool loops

These semantics should be consistent across billing, routing, and support surfaces so customers do not see one system reporting "within budget" while another is already rejecting traffic.

### 14.8 Cross-System Reason and State Consistency

Billing, routing, policy, and support surfaces should reuse:

- one budget status vocabulary
- one quota status vocabulary
- one reservation outcome vocabulary
- one set of customer-visible reason codes for materially equivalent denials

The customer should not see billing say "soft warning only" while routing says "hard reject" for the same evaluated state.

---

## 15. Pricing Engine

### 15.1 Requirements

The pricing engine must support:

- platform pricing catalogs
- provider cost tables
- customer price tables
- tenant-specific pricing overrides
- project-specific discounts or surcharges
- region-aware pricing
- modality-aware pricing
- time-based or event-based pricing
- cached token pricing
- image tier pricing

### 15.2 Price Resolution Order

1. custom contract price
2. tenant override price
3. plan-level price
4. default public price

### 15.2.1 Pricing Catalog Sources

Pricing data should identify where it came from. At minimum:

- `platform_catalog`: operator-maintained default catalog
- `provider_native`: vendor-published or vendor-derived upstream pricing
- `contract_override`: customer-specific negotiated pricing
- `tenant_override`: tenant-specific override that does not replace the underlying provider cost record
- `promotional`: temporary non-standard pricing treatment

Catalog-source metadata should remain available for diagnostics and finance workflows.

### 15.3 Cost vs Revenue

The system must clearly separate:

- **provider cost**
- **customer billable price**
- **internal transfer price**

This enables margin analytics and route optimization.

### 15.4 Budget and Pricing Interaction Rule

Budget enforcement should evaluate against the correct monetary view:

- provider cost when protecting shared upstream spend
- customer billable price when enforcing customer-facing contractual limits
- internal transfer price only for internal accounting or margin analysis, never as a hidden substitute for customer-facing policy

The enforced monetary view must be explicit in policy and route receipts.

### 15.5 Payment Collection Boundary

Payment collection sits downstream of pricing and ledger projections. External payment-provider state must never directly rewrite immutable usage or ledger history.

Rules:

- payment initiation creates a pending payment record, not a settled balance mutation
- balance or invoice settlement occurs only after callback verification and an explicit funding ledger entry
- refunds, charge reversals, and manual adjustments remain ledger-visible events
- the first customer-facing payment collection flow should support **WeChat Pay only**
- payment domain models and API shapes should stay provider-agnostic so additional rails can be added later without changing ledger semantics

---
