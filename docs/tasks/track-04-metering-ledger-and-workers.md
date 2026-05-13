# Track 04 - Metering, Ledger, And Workers

## Mission

Make background workers the durable operations layer for usage ingestion, ledger projections, route receipt diagnostics, provider health, notifications, audits, and routing intelligence.

## Current Baseline

- `ledger-worker` consumes `UsageEventRecordedMessage`, writes idempotent ledger entries, updates `usage_daily_projections`, updates `balance_projections`, and records budget threshold events.
- `route-receipt-worker` consumes `RouteReceiptRecordedMessage`, writes route receipts, and stores diagnostic timelines, policy checks, and provider attempts.
- `edge-probe` has a probe runtime shape and cheap-health mode, but health score feedback still needs stronger persistence and routing integration.
- `runtime-composition` provides shared startup conventions but still needs reusable worker configuration, NATS subscription helpers, tracing setup, shutdown handling, and health semantics.
- `audit-worker`, `notification-worker`, and `routing-worker` still need real operational responsibilities beyond the first scaffolding.
- Pricing and budget projections still rely on default catalog logic and should move toward control-plane managed policies.

## Owned Paths

- `crates/runtime-composition/*`
- `services/ledger-worker/*`
- `services/audit-worker/*`
- `services/notification-worker/*`
- `services/routing-worker/*`
- `services/edge-probe/*` when the change is about worker/runtime health orchestration

## Do Not Edit

- `services/gateway-api`
- `services/control-plane-api`
- `crates/core-domain`
- `crates/protocol-ir`
- `apps/console-web`

This track should consume emitted events and shared contracts, not redefine them.

## Deliverables

1. A reusable runtime bootstrap layer for worker configuration, NATS subscription, tracing, shutdown, and health semantics.
2. Ledger ingestion that uses control-plane managed pricing and supports reserve/finalize/release usage phases.
3. Audit ingestion that persists security, admin, budget, guardrail, and payload-access events with tenant scoping.
4. Routing-worker aggregation for provider latency, error rate, rate-limit, and circuit-breaker state.
5. Notification-worker fanout for budget thresholds, provider incidents, anomalous spend, and security events.
6. Edge-probe feedback that updates provider health/quarantine inputs consumed by route evaluation.

## Ordered Plan

1. Expand `runtime-composition`.
   - Add shared configuration loading, tracing setup, graceful shutdown, and worker bootstrap helpers.
   - Keep the runtime crate generic so multiple services can adopt it without hidden globals.
2. Upgrade ledger ingestion.
   - Replace default pricing assumptions with catalog lookups once Track `02` exposes managed catalog data.
   - Support reserve, partial, final, and release phases without double counting.
   - Preserve idempotency for retries and replay.
3. Implement audit processing.
   - Persist admin changes, auth events, budget rejections, provider failures, guardrail decisions, and sealed payload access.
   - Preserve request and trace correlation metadata from incoming envelopes.
4. Implement routing intelligence.
   - Aggregate route receipt and probe data into provider health, latency, error-rate, rate-limit, and quarantine signals.
   - Publish or persist those signals for gateway route evaluation.
5. Implement notification fanout.
   - Send budget threshold, provider incident, anomalous spend, and security events to configured webhook/email targets.
   - Deduplicate noisy alerts and record delivery attempts.
6. Add local integration wiring.
   - Use the existing local stack where appropriate.
   - Provide deterministic fixtures so later tracks can verify worker side effects.

## Required Tests

- Unit tests for runtime bootstrap helpers, handler logic, deduplication, and envelope validation.
- Integration tests for ledger ingestion, route receipt ingestion, audit ingestion, notification delivery attempts, and routing-health aggregation using real persistence boundaries or realistic test doubles.
- Tests for graceful shutdown and failure handling where shared runtime behavior is added.
- Event-contract tests proving worker inputs still match Track `01` envelope expectations.

## Definition Of Done

- Ledger and route receipt workers continue to process real events end-to-end.
- Shared runtime code exists and is reused by more than one service.
- Idempotency and malformed-event behavior are covered by tests.
- Audit, notification, routing, and probe responsibilities are explicit and documented rather than implied by service names.
- Future operational work can build on these workers without replacing their startup model from scratch.

## Branch And PR Convention

- Branch: `codex/track-04-workers-metering`
- PR title: `[Track 04] Implement metering and worker runtime flows`
- Required PR notes:
  - which workers became real in this PR
  - how idempotency is enforced
  - which local dependencies are required for verification
