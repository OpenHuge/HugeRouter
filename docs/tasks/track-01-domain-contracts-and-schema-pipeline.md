# Track 01 - Domain Contracts And Schema Pipeline

## Mission

Turn the existing domain and protocol scaffolding into a stable contract layer that both Rust services and TypeScript consumers can build against without hand-written placeholders.

## Current Baseline

- `crates/core-domain` already defines prefixed IDs, core enums, and serialized records for provider resources, route receipts, usage events, ledger entries, audit events, and replay capsules.
- `crates/protocol-ir` already provides request and message envelopes plus a few event payloads.
- `packages/ts-shared-schema` contains only `tenantSchema`.
- `packages/ts-api-client` is a handwritten placeholder client returning mock data.
- `schemas/openapi`, `schemas/jsonschema`, and `schemas/examples` exist, but there is no real generation pipeline yet.

## Owned Paths

- `crates/core-domain/*`
- `crates/protocol-ir/*`
- `schemas/openapi/*`
- `schemas/jsonschema/*`
- `schemas/examples/*`
- `packages/ts-shared-schema/*`
- `packages/ts-api-client/*`
- generation scripts created specifically for this contract pipeline

## Do Not Edit

- `services/control-plane-api`
- `services/gateway-api`
- `apps/console-web`
- `packages/ui-kit`
- `packages/test-utils`

Other tracks should consume the contracts you publish here instead of redefining them locally.

## Deliverables

1. Stable v1 Rust domain and protocol types for the first real gateway and control-plane slice.
2. OpenAPI and JSON Schema artifacts that match those contracts.
3. Generated or contract-backed TypeScript schemas and API client modules.
4. Examples and fixtures that support both backend and frontend tests.
5. Clear versioning rules for evolving contracts without silent breakage.

## Ordered Plan

1. Freeze the first stable scope.
   - Decide the minimal contract set needed by Tracks `02`, `03`, and `05`.
   - Keep the scope to the current MVP path: tenants, projects, provider resources, route policies, config snapshots, chat requests, route receipts, usage events, and normalized errors.
2. Normalize Rust contracts.
   - Review current serialization names, optional fields, and envelope shapes.
   - Replace bootstrap-only naming where it leaks into public-facing types.
   - Add semantic validation helpers where raw structs are not enough.
3. Establish schema sources of truth.
   - Define which artifacts are authored versus generated.
   - Add repeatable generation commands under the workspace command surface.
   - Check in fixtures and examples that prove the contract is usable.
4. Replace placeholder TypeScript packages.
   - Expand `ts-shared-schema` to expose the same validated entities the frontend needs.
   - Replace the placeholder `ts-api-client` with typed modules derived from real contracts.
   - Keep the public API small and stable enough for `apps/console-web` to consume directly.
5. Document compatibility rules.
   - State how breaking versus additive contract changes are identified.
   - Add guidance for future tracks that need new fields or new protocol families.

## Required Tests

- Rust unit tests for parsing, validation, serialization, and round-trip behavior in `core-domain` and `protocol-ir`.
- Snapshot or golden tests for OpenAPI and JSON Schema outputs.
- TypeScript tests for Zod or schema validation behavior.
- TypeScript tests for generated or contract-backed client request and response typing.
- One contract-consistency test that proves the checked-in schema artifacts and code-generated packages are synchronized.

## Definition Of Done

- No consumer needs to hand-write placeholder contract objects for the MVP flow.
- Rust and TypeScript contracts are generated or maintained from one documented source-of-truth workflow.
- Examples in `schemas/examples` cover the first real control-plane and gateway flows.
- Contract tests fail when serialization shape changes unintentionally.
- Follow-on tracks can depend on these packages without editing them locally.

## Branch And PR Convention

- Branch: `codex/track-01-contracts-schema`
- PR title: `[Track 01] Stabilize domain contracts and schema pipeline`
- Required PR notes:
  - which contracts are now considered stable
  - the exact generation command
  - any intentionally deferred protocol families
