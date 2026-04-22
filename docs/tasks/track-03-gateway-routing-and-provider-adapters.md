# Track 03 - Gateway, Routing, And Provider Adapters

## Mission

Convert `gateway-api` from a bootstrap placeholder into the first real northbound execution path, backed by typed provider adapters and route selection logic.

## Current Baseline

- `services/gateway-api` already exposes `/healthz` and `/v1/chat/completions`.
- The current implementation validates bearer auth and basic request shape, then synthesizes a `ConfigSnapshot`, `RouteReceipt`, `UsageEvent`, and `MessageEnvelope` before returning a placeholder assistant response.
- `crates/provider-traits` is still a minimal trait returning `Result<String>`.
- There is no real upstream adapter, no real routing engine, and no persistence-backed configuration lookup yet.

## Owned Paths

- `services/gateway-api/*`
- `crates/provider-traits/*`
- new gateway or provider crates created for this track, such as:
  - routing engine crates
  - OpenAI adapter crates
  - protocol translation crates for gateway ingress and egress

## Do Not Edit

- `crates/core-domain`
- `crates/protocol-ir`
- `services/control-plane-api`
- `apps/console-web`
- `packages/ts-api-client`
- `packages/ts-shared-schema`

If a required contract change is discovered, land it through Track `01` and then rebase.

## Deliverables

1. A real request normalization path for the first OpenAI-compatible northbound flow.
2. Typed provider adapter contracts that model success, failure, usage, and streaming boundaries.
3. Route selection that consumes configured provider targets instead of hard-coded bootstrap values.
4. Error mapping that preserves internal signal while returning normalized client-facing responses.
5. Tests that cover both happy path and failure path HTTP behavior.

## Ordered Plan

1. Replace placeholder execution seams.
   - Separate HTTP parsing, request normalization, route selection, provider execution, metering extraction, and response mapping into explicit layers.
   - Keep the existing endpoint surface stable while replacing the internals.
2. Redesign provider traits.
   - Replace `Result<String>` with typed request, response, usage, and error structures.
   - Make adapter contracts testable without real network calls.
3. Implement the first upstream adapter.
   - Build one real provider adapter, most likely OpenAI first.
   - Support non-streaming requests before adding streaming complexity.
4. Add route selection.
   - Consume active config from Track `02`.
   - Produce real `RouteReceipt`, score breakdown, exclusions, and fallback reasons from route evaluation instead of synthetic constants.
5. Add streaming only after the non-streaming path is solid.
   - Implement SSE relay and client cancellation handling once the provider boundary is typed and tested.

## Required Tests

- Unit tests for request normalization, route scoring, adapter error mapping, and usage extraction.
- HTTP integration tests for `/healthz` and `/v1/chat/completions`.
- Mock-adapter tests proving route selection and fallback behavior without a live upstream.
- Streaming tests if SSE lands in the same PR, including disconnect and partial-event coverage.
- Regression tests that compare responses and normalized errors against Track `01` examples.

## Definition Of Done

- The first gateway flow is no longer purely synthetic.
- A typed provider adapter interface exists and is used by the gateway instead of ad hoc placeholder strings.
- Route receipts are derived from actual route evaluation inputs.
- Gateway tests cover authentication failure, validation failure, provider failure, and successful completion.
- Follow-on work can add new adapters without editing the gateway handler directly.

## Branch And PR Convention

- Branch: `codex/track-03-gateway-routing`
- PR title: `[Track 03] Implement gateway routing and provider adapters`
- Required PR notes:
  - which northbound flow is fully real
  - whether streaming is included or intentionally deferred
  - sample request and response fixtures used in verification
