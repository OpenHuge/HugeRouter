# Risk Burndown Plan

[Back to Execution Index](README.md)

## Top execution risks

### 1. Shared contract churn
**Risk:** `protocol-ir`, adapter traits, storage models, or OpenAPI documents change too often and break parallel streams.  
**Mitigation:** assign temporary interface owners and require example fixtures before large refactors.

### 2. Monorepo friction
**Risk:** build times and CI times slow down as crates and packages grow.  
**Mitigation:** selective CI, workspace caching, feature-gated tests, and clear package boundaries.

### 3. Gateway hot-path instability
**Risk:** the MVP gateway works functionally but lacks safe retry, backpressure handling, or observability.  
**Mitigation:** add streaming tests, route explanation traces, and bounded retries before broad rollout.

### 4. Ledger correctness errors
**Risk:** pricing and projection logic diverge from source usage events.  
**Mitigation:** immutable append-only ledger, replay jobs, idempotency keys, and pricing simulation endpoints.

### 5. Secret leakage
**Risk:** provider credentials leak through APIs, logs, or generated clients.  
**Mitigation:** secret reference model, explicit redaction helpers, review gates on sensitive paths.

### 6. Too much frontend waiting on backend
**Risk:** the console team stalls while backend APIs are still unstable.  
**Mitigation:** lock OpenAPI conventions early, generate TypeScript clients, and use seeded mocks for non-critical routes.

## Risk burndown checkpoints

- End of PI-0: interface churn is controlled and CI works
- End of PI-1: gateway MVP is stable enough for smoke traffic
- End of PI-2: route reliability and diagnostics reduce operational uncertainty
- End of PI-3: billing accuracy and SSO are validated in integration environments
- End of PI-4: ecosystem integrations do not compromise core path stability
