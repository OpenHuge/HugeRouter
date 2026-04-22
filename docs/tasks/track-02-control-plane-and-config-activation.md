# Track 02 - Control Plane And Config Activation

## Mission

Turn `control-plane-api` from a startup placeholder into the authoritative source for administrative data and activated configuration consumed by the gateway.

## Current Baseline

- `services/control-plane-api` only initializes tracing and logs startup.
- Domain types for tenants, projects, provider resources, route policies, budget policies, and config snapshots already exist in `crates/core-domain`.
- Infrastructure for PostgreSQL, Redis, and NATS exists in `infra/docker/compose.yaml`, but no storage layer or migrations are implemented in this service yet.
- The frontend currently uses placeholder tenancy and project data because there is no real API surface to call.

## Owned Paths

- `services/control-plane-api/*`
- new backend support crates created specifically for this track, such as:
  - configuration loading
  - storage and repositories
  - authn/authz middleware for control-plane APIs
  - migration or seed utilities owned by the control plane

## Do Not Edit

- `crates/core-domain`
- `crates/protocol-ir`
- `services/gateway-api`
- `apps/console-web`
- `packages/ts-api-client`
- `packages/ts-shared-schema`

If Track `02` needs contract changes, request them through Track `01` instead of redefining contracts locally.

## Deliverables

1. A real HTTP API with health endpoints and consistent error responses.
2. Typed configuration loading and fail-fast startup validation.
3. Persistence primitives and migrations for the first administrative data model.
4. CRUD or read/write flows for tenants, projects, provider resources, route policies, and config snapshots.
5. An activation path that publishes or records the currently active configuration snapshot.

## Ordered Plan

1. Build the service skeleton for real work.
   - Add router setup, health/readiness endpoints, config loading, and shared middleware.
   - Establish error handling and request IDs early so later handlers inherit a stable surface.
2. Add storage boundaries.
   - Introduce repositories and migrations for the minimum viable administrative model.
   - Keep repository traits small enough for unit testing without a full database in every case.
3. Implement the first control-plane resources.
   - Start with tenants and projects.
   - Add provider resources and route policies next because the gateway cannot stop using bootstrap constants until these exist.
4. Implement config snapshot activation.
   - Persist snapshot revisions.
   - Expose an activation endpoint or command path that makes the active snapshot discoverable.
   - Ensure the gateway can later consume this without coupling to ad hoc storage details.
5. Seed and demo support.
   - Add a deterministic bootstrap seed path suitable for local development and integration tests.
   - Keep fixtures aligned with Track `01` schemas so frontend and gateway tests can reuse them.

## Required Tests

- Unit tests for config validation, repository behavior, and authorization rules.
- Handler-level tests for success, validation failure, not-found, and conflict cases.
- Database integration tests covering migrations and the first CRUD flows.
- Tests for config snapshot activation semantics, including revision handling and idempotency expectations.
- Contract tests that verify responses conform to the schemas published by Track `01`.

## Definition Of Done

- `control-plane-api` serves more than a startup log and exposes a real administrative surface.
- The gateway no longer has to depend on hard-coded bootstrap entities as the only source of config.
- The first data model is persisted through migrations rather than in-memory placeholders.
- Failure cases are normalized and documented by tests.
- Local seeded data is deterministic enough for frontend and gateway integration work.

## Branch And PR Convention

- Branch: `codex/track-02-control-plane`
- PR title: `[Track 02] Implement control plane and config activation`
- Required PR notes:
  - the exact resources implemented
  - migration strategy and rollback notes
  - the seeded demo data or fixtures included
