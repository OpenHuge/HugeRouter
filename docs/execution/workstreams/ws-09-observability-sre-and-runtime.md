# Observability, SRE, and Runtime

[Back to Execution Workstreams](README.md)

Create shared telemetry, SLOs, alerting, and performance/failure-injection practices.

## Task sequence

| Task ID | Phase | Title                                                                 | Depends On                |
| ------- | ----- | --------------------------------------------------------------------- | ------------------------- |
| OBS-001 | PI-0  | Implement telemetry crate and platform-wide OpenTelemetry conventions | FND-001, FND-003          |
| OBS-002 | PI-1  | Build service-level dashboards and SLO definitions                    | OBS-001, GWT-001, CTL-001 |
| OBS-003 | PI-2  | Implement alert routing and incident notification pipeline            | OBS-002, SEC-004          |
| OBS-004 | PI-3  | Create load, soak, and failure-injection test suite                   | GWT-005, RTE-004, MET-002 |

## Detailed tasks

### OBS-001 — Implement telemetry crate and platform-wide OpenTelemetry conventions

**Phase:** PI-0  
**Estimated size:** M  
**Recommended owners:** sre

**Depends on:** FND-001, FND-003

**Primary paths to touch:**

- `crates/telemetry`
- `infra/monitoring`
- `services/*`

**Expected outputs:**

- trace helpers
- metrics registry
- structured log conventions

**Acceptance criteria:**

- all services emit traces, metrics, and logs with shared resource tags
- trace IDs propagate across HTTP and events
- collector config is documented

**Implementation notes:**

- Tracing, logging, and metrics conventions must be shared across services.
- Runbooks should link directly from dashboards or alert metadata.
- Test degraded dependencies deliberately, not only happy paths.

### OBS-002 — Build service-level dashboards and SLO definitions

**Phase:** PI-1  
**Estimated size:** M  
**Recommended owners:** sre

**Depends on:** OBS-001, GWT-001, CTL-001

**Primary paths to touch:**

- `infra/monitoring`
- `docs/runbooks`

**Expected outputs:**

- Grafana dashboards
- latency/error/availability SLOs
- runbooks

**Acceptance criteria:**

- dashboards exist for gateway, control plane, and workers
- SLO calculations are reproducible
- runbooks link alerts to operational actions

**Implementation notes:**

- Tracing, logging, and metrics conventions must be shared across services.
- Runbooks should link directly from dashboards or alert metadata.
- Test degraded dependencies deliberately, not only happy paths.

### OBS-003 — Implement alert routing and incident notification pipeline

**Phase:** PI-2  
**Estimated size:** S  
**Recommended owners:** sre

**Depends on:** OBS-002, SEC-004

**Primary paths to touch:**

- `infra/monitoring`
- `services/notification-worker`

**Expected outputs:**

- alert rules
- notification fan-out
- severity taxonomy

**Acceptance criteria:**

- high-severity incidents trigger notifications
- alert noise is rate-limited
- test alerts can be fired safely

**Implementation notes:**

- Tracing, logging, and metrics conventions must be shared across services.
- Runbooks should link directly from dashboards or alert metadata.
- Test degraded dependencies deliberately, not only happy paths.

### OBS-004 — Create load, soak, and failure-injection test suite

**Phase:** PI-3  
**Estimated size:** L  
**Recommended owners:** sre, qa

**Depends on:** GWT-005, RTE-004, MET-002

**Primary paths to touch:**

- `crates/testing-kit`
- `infra/scripts`
- `docs/runbooks`

**Expected outputs:**

- performance test scenarios
- failure injection harness
- capacity baseline report

**Acceptance criteria:**

- test suite covers concurrency, retry storms, and degraded dependencies
- results are versioned and comparable
- capacity thresholds are documented

**Implementation notes:**

- Tracing, logging, and metrics conventions must be shared across services.
- Runbooks should link directly from dashboards or alert metadata.
- Test degraded dependencies deliberately, not only happy paths.
