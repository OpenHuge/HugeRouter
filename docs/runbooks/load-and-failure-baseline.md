# Load And Failure Baseline

[Back to Docs Index](../README.md)

This runbook defines the lightweight PI-3 performance and failure-injection baseline.

## Load baseline

- `infra/scripts/load-baseline.sh usage-summary`
- `infra/scripts/load-baseline.sh usage-breakdown`
- `GATEWAY_API_KEY=... infra/scripts/load-baseline.sh gateway-chat`
- `SOAK_SECONDS=300 infra/scripts/load-baseline.sh soak-usage-summary`
- `SOAK_SECONDS=300 infra/scripts/load-baseline.sh soak-usage-breakdown`

Useful environment overrides:

- `CONCURRENCY`
- `ITERATIONS`
- `TENANT_ID`
- `CONTROL_PLANE_BASE_URL`
- `GATEWAY_BASE_URL`

Record for each run:

- scenario
- timestamp
- concurrency
- iterations
- results file path
- p50 / p95 if measured externally
- observed failures

## Failure injection

- `infra/scripts/failure-injection.sh gateway-auth-rejection`
- `infra/scripts/failure-injection.sh control-plane-missing-tenant`
- `infra/scripts/failure-injection.sh control-plane-breakdown-invalid-group`
- `infra/scripts/failure-injection.sh retry-storm`

These scenarios intentionally exercise:

- gateway auth rejection
- control-plane validation failures
- usage analytics parameter validation
- retry storm style repeated failure handling

## Capacity thresholds

Use these initial PI-3 thresholds until a later runbook revises them:

- usage summary/read-model endpoints should stay below 500ms at the current local baseline
- retry-storm validation scenarios should not crash services or grow unbounded error output
- projection lag should remain visible in the billing UI and not silently disappear

## Baseline report template

```text
Scenario:
Environment:
Concurrency:
Iterations:
Observed status codes:
Latency notes:
Projection lag notes:
Follow-up actions:
```
