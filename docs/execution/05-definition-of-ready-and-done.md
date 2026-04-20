# Definition of Ready and Definition of Done

[Back to Execution Index](README.md)

## Definition of Ready

A task is ready when:

- its upstream dependencies are merged or stubbed
- file ownership is clear
- inputs and outputs are specified
- acceptance criteria are testable
- required schemas, migrations, or config variables are identified
- any hidden feature flags are specified if the feature is not fully exposed yet

## Definition of Done

A task is done when:

- code compiles and local checks pass
- tests for the touched behavior are added or updated
- generated artifacts are committed if repository policy requires them
- docs are updated when a user-facing or operator-facing workflow changes
- metrics, traces, or logs exist for operationally meaningful features
- security-sensitive changes emit audit events where applicable
- no plaintext secrets or environment-specific values are committed

## Done-plus criteria for production features

For hot-path gateway and billing features, done also means:

- latency or throughput impact was considered
- idempotency and replay behavior were defined
- degraded dependency behavior was tested
- failure modes produce normalized errors and diagnostic context
