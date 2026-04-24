# Track 02 - Control Plane And Config Activation

## Mission

Evolve `control-plane-api` from a functional administrative surface into the authoritative operations plane for cost policy, route policy, provider capability metadata, tenant governance, and activation state consumed by the gateway and console.

## Current Baseline

- `services/control-plane-api` exposes real HTTP routes for health, auth providers, email/OAuth login, current session, provider links, tenants, projects, provider resources, route policies, config snapshots, API keys, usage, billing, route simulations, route receipts, diagnostics, merchant workspaces, trial connections, relay evaluations, and replay capsules.
- The service supports memory and Postgres store modes. SQL bootstrap and schema files cover control-plane resources, sessions, route receipt diagnostics, ledger/projection tables, and billing export jobs.
- Config activation is persisted through `config_snapshots` and `active_config_pointers`, and the gateway consumes `/internal/gateway/config/current`.
- API key resolution and budget projection are available through internal gateway endpoints guarded by `CONTROL_PLANE_INTERNAL_TOKEN`.
- Authorization now filters tenant-scoped reads and writes by session membership, with platform-admin global access.
- The next gaps are policy depth: persistent pricing catalogs, editable budget policies, richer role scopes, provider capability inventory, guardrail configuration, retention/redaction policy, and channel/white-label commercial controls.

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

1. Editable pricing catalogs and budget policies that can be activated through config snapshots rather than compiled defaults.
2. Pre-admission budget projection APIs that can estimate, reserve, release, and finalize spend for short and long-lived requests.
3. Provider and model capability inventory APIs consumed by route policy authoring and gateway admission.
4. Route policy APIs that model strategy, rate windows, fallback conditions, circuit breakers, and tenant/customer tier constraints.
5. Guardrail and redaction policy APIs with retention, audit, and access controls.
6. Stronger enterprise administration for roles, tenant data boundaries, channel pricing, and audit export.

## Ordered Plan

1. Replace pricing defaults with control-plane managed catalog data.
   - Persist catalog entries by provider, model alias, region, usage dimension, unit denominator, rate source, and effective window.
   - Keep a deterministic bootstrap catalog for local development, but make the production path data-driven.
2. Add first-class budget policy resources.
   - Support tenant, project, API key, user, app, daily, and monthly scopes.
   - Expose threshold configuration and exceeded behavior: reject, degrade route, require approval, or alert-only.
3. Add gateway pre-admission support.
   - Return estimated cost, remaining budget, threshold status, and reserve token from an internal gateway endpoint.
   - Make reserve/finalize idempotent so retries do not double spend.
4. Add provider/model capability inventory.
   - Store capabilities from adapter manifests and manual overrides.
   - Expose capability matrix endpoints for console route authoring and route diagnostics.
5. Add reliability and guardrail policy resources.
   - Persist route strategy, fallback conditions, rate windows, circuit breaker thresholds, redaction tier, payload capture, and guardrail chain settings.
   - Make activation snapshot validation reject policies that reference missing providers, models, budgets, or guardrails.
6. Harden enterprise administration.
   - Expand RBAC from tenant admin/member to capability-specific permissions.
   - Add audit export and retention-policy surfaces.
   - Add channel pricing and settlement metadata without mixing it into provider cost source-of-truth tables.

## Required Tests

- Unit tests for config validation, repository behavior, and authorization rules.
- Handler-level tests for success, validation failure, not-found, and conflict cases.
- Database integration tests covering migrations, managed pricing, budget policies, reservations, and activation validation.
- Tests for config snapshot activation semantics, including revision handling, compatibility checks, and idempotency expectations.
- Contract tests that verify responses conform to the schemas published by Track `01`.
- Auth handler tests for email login, OAuth callback success/failure, session lookup, logout, account linking, and tenant membership resolution.
- Persistence tests for user, provider-link, and session lifecycle behavior.

## Definition Of Done

- Cost, budget, route, capability, guardrail, and retention policy are controlled by data, not by env-only defaults or code constants.
- Gateway admission can ask the control plane for deterministic estimate/reserve/finalize decisions.
- Config activation validates the full policy graph before making it active.
- Console auth continues to work against backend-owned identity and session flows.
- Failure cases are normalized and documented by tests.
- Local seeded data is deterministic enough for frontend and gateway integration work.

## Branch And PR Convention

- Branch: `codex/track-02-control-plane`
- PR title: `[Track 02] Implement control plane and config activation`
- Required PR notes:
  - the exact resources implemented
  - which auth flows and providers were implemented
  - migration strategy and rollback notes
  - the seeded demo data or fixtures included
