# Track 01 - Domain Contracts And Schema Pipeline

## Mission

Keep the Rust, schema, and TypeScript contract layer stable as HugeRouter moves from a working gateway MVP toward an AI traffic control plane with cost governance, routing diagnostics, model capabilities, guardrails, and enterprise administration.

## Current Baseline

- `crates/core-domain` defines prefixed IDs, provider resources, provenance classes, route policies, config snapshots, route receipts, usage events, ledger entries, auth/session records, merchant relay records, replay capsules, and related enums.
- `crates/protocol-ir` defines gateway/control-plane DTOs, protocol families, event envelopes, route receipt diagnostics, usage/billing/pricing responses, route simulations, and OpenAPI/JSON Schema generation helpers.
- `schemas/openapi`, `schemas/jsonschema`, and `schemas/examples` are generated and checked in via `pnpm generate`.
- `packages/ts-shared-schema` exposes meaningful Zod schemas for control-plane and merchant resources.
- `packages/ts-api-client` exposes a real fetch-based control-plane client with tests for request construction and error handling.
- The next contract gaps are not the MVP basics; they are richer pricing policy, pre-admission/reserve accounting, model capability matrices, fallback/guardrail decisions, redaction policy, and enterprise/channel governance shapes.

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

1. Stable v1 Rust domain and protocol types for the shipped gateway and control-plane surface.
2. OpenAPI and JSON Schema artifacts that match those contracts.
3. Contract-backed TypeScript schemas and API client modules.
4. Examples and fixtures that support backend, frontend, and worker tests.
5. Clear versioning rules for evolving contracts without silent breakage.
6. New contract families for cost governance, model capabilities, reliability/fallback policy, redaction, guardrails, and enterprise/channel administration.

## Ordered Plan

1. Extend cost governance contracts.
   - Add explicit pricing catalog records for provider, model, region, token direction, cached token, image, audio, and channel/customer price dimensions.
   - Add budget policy records for tenant, project, credential, user, app, daily, and monthly scopes.
   - Add reserve/release usage phases that can support request pre-admission and long-lived sessions without double counting.
2. Extend routing and reliability contracts.
   - Add policy fields for cost-first, latency-first, quality-first, availability-first, customer-tier, and region constrained strategies.
   - Add circuit-breaker, concurrency, rate-window, and fallback condition records.
   - Preserve route receipt explainability with score breakdowns, exclusions, fallback transitions, provider attempts, policy checks, and validation outcomes.
3. Add model capability contracts.
   - Represent per-provider and per-model support for streaming, tool calling, JSON mode, JSON schema, long context, vision, images, audio, embeddings, rerank, batch, realtime, and response metadata.
   - Model unsupported capability reasons and downgrade guidance instead of flattening provider differences.
4. Add redaction and guardrail contracts.
   - Define redaction tier, payload capture policy, PII finding summaries, prompt-injection findings, content safety decisions, output validation results, and audit references.
   - Keep raw prompt retention opt-in and separable from ordinary route diagnostics.
5. Document compatibility rules.
   - State how breaking versus additive contract changes are identified.
   - Add guidance for future tracks that need new fields or new protocol families.
   - Document how new auth providers, guardrail providers, and capability dimensions are added without breaking existing enums or callback payloads.

## Required Tests

- Rust unit tests for parsing, validation, serialization, and round-trip behavior in `core-domain` and `protocol-ir`.
- Snapshot or golden tests for OpenAPI and JSON Schema outputs.
- TypeScript tests for Zod or schema validation behavior.
- TypeScript tests for generated or contract-backed client request and response typing.
- One contract-consistency test that proves the checked-in schema artifacts and code-generated packages are synchronized.
- Contract tests for auth-provider enums, session payloads, provider-link serialization, and email versus OAuth callback request shapes.

## Definition Of Done

- No consumer needs to hand-write placeholder contract objects for the shipped gateway, control-plane, auth, merchant, usage, billing, and route diagnostic flows.
- Rust and TypeScript contracts are generated or maintained from one documented source-of-truth workflow.
- Examples in `schemas/examples` cover shipped control-plane, gateway, auth, usage, billing, and event flows.
- Contract tests fail when serialization shape changes unintentionally.
- Follow-on tracks can depend on these packages without editing them locally.

## Branch And PR Convention

- Branch: `codex/track-01-contracts-schema`
- PR title: `[Track 01] Stabilize domain contracts and schema pipeline`
- Required PR notes:
  - which contracts are now considered stable
  - the exact generation command
  - any intentionally deferred protocol families
