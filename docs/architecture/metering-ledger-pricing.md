# 14. Metering and Ledger

[Back to Docs Index](../README.md)

### 14.1 Design Principle

Metering records what happened. Billing interprets what it means commercially.

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
- vector operation units if added in the future

### 14.3 Usage Event Flow

1. request starts
2. route selected
3. upstream emits partial or final response (or continuous frames in a `ws` stream)
4. adapter extracts usage fields
5. **Progressive Metering:** For long-lived WebSocket/Realtime sessions, adapters emit partial normalized usage events periodically (e.g., every minute or per tool-call) rather than waiting for connection termination. This is critical for interrupting runaway agent loops before they cause a "Denial of Wallet".
6. normalized usage event emitted to message bus
7. ledger worker persists immutable usage event
8. projection tables update balances, cost summaries, and analytics, triggering circuit breakers if quotas are exceeded.

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
- anomaly score

### 14.6 Idempotency

Ledger ingestion must be idempotent. Duplicate usage events must not double-charge.


---


## 15. Pricing Engine

### 15.1 Requirements

The pricing engine must support:

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

### 15.3 Cost vs Revenue

The system must clearly separate:

- **provider cost**
- **customer billable price**
- **internal transfer price**

This enables margin analytics and route optimization.

---
