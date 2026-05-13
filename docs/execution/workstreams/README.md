# Workstreams

[Back to Execution Index](../README.md)

- [Foundation and Monorepo](ws-01-foundation-and-monorepo.md) - Establish repository, tooling, local runtime, schema generation, and CI. This stream unlocks nearly every other stream but can itself be parallelized into Rust workspace, frontend shell, infra stack, and CI automation.
- [Data Storage and Events](ws-02-data-storage-and-events.md) - Build the relational schema, migration strategy, repository layer, and asynchronous event bus foundation used by ledger, audit, and routing workers.
- [Gateway and Protocol Ingress](ws-03-gateway-and-protocol-ingress.md) - Implement the northbound API gateway, protocol IR, protocol-specific ingress layers, and streaming/realtime request handling.
- [Provider Adapters](ws-04-provider-adapters.md) - Implement upstream provider integration through shared traits, error normalization, conformance tests, and individual adapters.
- [Routing and Reliability](ws-05-routing-and-reliability.md) - Own route domain models, route selection, health signals, retries, circuit breaking, fallback, and operator diagnostics.
- [Control Plane and Console](ws-06-control-plane-and-console.md) - Deliver the management API and the TanStack Start + HeroUI console for tenants, providers, routes, and billing surfaces.
- [Metering, Ledger, and Billing](ws-07-metering-ledger-and-billing.md) - Capture usage on the hot path, write immutable ledger records, compute projections, and expose billing analytics.
- [Security, Identity, and Compliance](ws-08-security-identity-and-compliance.md) - Define authorization, secret boundaries, SSO, audit trails, and compliance controls.
- [Observability, SRE, and Runtime](ws-09-observability-sre-and-runtime.md) - Create shared telemetry, SLOs, alerting, and performance/failure-injection practices.
- [QA, Release, and Documentation](ws-10-qa-release-and-documentation.md) - Build contract tests, e2e test environments, release automation, and agent-facing implementation guidance.
