# Track 04 - Metering, Ledger, And Workers

## Mission

Turn the current worker placeholders into a real background-processing layer for usage ingestion, ledger persistence, audit capture, and shared runtime conventions.

## Current Baseline

- `services/ledger-worker`, `services/audit-worker`, `services/notification-worker`, and `services/routing-worker` currently only initialize tracing and announce startup.
- `crates/runtime-composition` currently provides only a startup log helper.
- `core-domain` already contains `UsageEvent`, `LedgerEntry`, `AuditEvent`, `ReplayCapsule`, and related IDs.
- `protocol-ir` already has a `UsageEventRecorded` envelope type, but nothing consumes it yet.

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

1. A reusable runtime bootstrap layer for worker configuration, tracing, shutdown, and health semantics.
2. Real usage-event ingestion in `ledger-worker` with idempotent processing.
3. Real audit-event handling in `audit-worker`.
4. A clear boundary for notifications and routing health work, even if their first version is intentionally small.
5. Tests proving event handling and worker logic are deterministic.

## Ordered Plan

1. Expand `runtime-composition`.
   - Add shared configuration loading, tracing setup, graceful shutdown, and worker bootstrap helpers.
   - Keep the runtime crate generic so multiple services can adopt it without hidden globals.
2. Implement ledger ingestion first.
   - Build the pipeline from `UsageEventRecorded` to persisted ledger state.
   - Handle duplicates, malformed payloads, and replay safety explicitly.
3. Implement audit processing next.
   - Define the first useful audit events and persistence or fanout behavior.
   - Preserve request and trace correlation metadata from the incoming envelopes.
4. Right-size notification and routing workers.
   - Give them explicit first responsibilities instead of empty startup binaries.
   - Prefer narrow, tested behavior over speculative framework code.
5. Add local integration wiring.
   - Use the existing local stack where appropriate.
   - Provide deterministic fixtures so later tracks can verify worker side effects.

## Required Tests

- Unit tests for runtime bootstrap helpers, handler logic, deduplication, and envelope validation.
- Integration tests for ledger ingestion and audit ingestion using real persistence boundaries or realistic test doubles.
- Tests for graceful shutdown and failure handling where shared runtime behavior is added.
- Event-contract tests proving worker inputs still match Track `01` envelope expectations.

## Definition Of Done

- At least one worker path processes real events end-to-end instead of only logging startup.
- Shared runtime code exists and is reused by more than one service.
- Idempotency and malformed-event behavior are covered by tests.
- Worker responsibilities are explicit and documented rather than implied by placeholder service names.
- Future operational work can build on these workers without replacing their startup model from scratch.

## Branch And PR Convention

- Branch: `codex/track-04-workers-metering`
- PR title: `[Track 04] Implement metering and worker runtime flows`
- Required PR notes:
  - which workers became real in this PR
  - how idempotency is enforced
  - which local dependencies are required for verification
