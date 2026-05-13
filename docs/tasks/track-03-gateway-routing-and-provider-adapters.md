# Track 03 - Gateway, Routing, And Provider Adapters

## Mission

Turn `gateway-api` from a working multi-provider gateway into the hot-path enforcement layer for admission control, model capability matching, intelligent routing, fallback, guardrails, and redaction-first diagnostics.

## Current Baseline

- `services/gateway-api` exposes `/healthz`, adapter manifest lookup endpoints, `/v1/chat/completions`, `/v1/responses`, `/v1/images/generations`, `/v1/messages`, and Gemini-compatible generateContent ingress.
- The gateway resolves bearer API keys, active config, and budget projection from `control-plane-api`.
- The provider boundary uses typed request/response/error/usage structures, adapter manifests, image execution support, transit metadata, and normalized provider errors.
- Built-in adapters cover OpenAI, Anthropic, Gemini, Bedrock Converse, OpenAI-compatible transit gateways, and `chatgpt_web`.
- Route evaluation filters by route policy, model alias, provider status, health, protocol family, capabilities, and configured snapshot membership; then scores latency, cost, health, trust, region, transit hops, and priority.
- Retryable provider errors can fallback to the next ranked target. Route receipts include exclusions, score breakdown, fallback transitions, provider attempts, and normalized errors.
- The next gaps are dynamic routing intelligence, request pre-admission reservation, streaming parity, structured-output validation, circuit breaking, rate-limit awareness, guardrail stages, and module size/ownership cleanup.

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

1. Gateway-side request estimate/reserve/finalize flow against the control plane before upstream execution.
2. Route evaluation extracted into a reusable routing engine with strategy-aware scoring and testable policy inputs.
3. Dynamic health, latency, rate-limit, and circuit-breaker inputs from workers/probes.
4. Fallback conditions beyond provider retryability: status code, timeout, JSON schema validation, guardrail result, output format, and customer tier.
5. Streaming parity for supported protocol families, including usage finalization and partial failure diagnostics.
6. Guardrail and redaction stages that are part of the request pipeline, not external afterthoughts.

## Ordered Plan

1. Split hot-path modules before adding more policy complexity.
   - Move route evaluation, route receipt construction, adapter manifest presentation, normalization, budget admission, and provider execution into owned modules or crates.
   - Preserve endpoint behavior while shrinking `services/gateway-api/src/lib.rs`.
2. Add budget reserve admission.
   - Estimate max request cost from model, protocol, expected prompt tokens, max output tokens, and modality.
   - Call the control-plane reserve endpoint before upstream execution and finalize/release after completion or failure.
3. Upgrade routing strategy.
   - Add cost-first, latency-first, availability-first, trust-first, and customer-tier strategy knobs.
   - Consume live health/latency/rate-limit state from worker outputs rather than static resource fields only.
4. Expand fallback semantics.
   - Add fallback conditions for status class, timeout, structured output validation failure, guardrail rejection, and provider capability downgrade.
   - Record each fallback decision in route receipts with enough detail to explain why it happened.
5. Add guardrail pipeline stages.
   - Run pre-request checks before provider execution and post-response checks before final response mapping.
   - Emit redacted diagnostics and audit events for guardrail decisions.
6. Add streaming after admission and fallback rules are stable.
   - Relay SSE safely.
   - Track client disconnects, partial usage, reserve release, and trace continuity.

## Required Tests

- Unit tests for request normalization, route scoring, adapter error mapping, usage extraction, admission estimates, fallback policy, and guardrail decisions.
- HTTP integration tests for all shipped gateway protocol families.
- Mock-adapter tests proving route selection and fallback behavior without a live upstream.
- Streaming tests if SSE lands in the same PR, including disconnect and partial-event coverage.
- Regression tests that compare responses, route receipts, normalized errors, and usage events against Track `01` examples.

## Definition Of Done

- Gateway admission enforces budget and quota before upstream traffic.
- Routing decisions can explain strategy inputs, live health inputs, exclusions, score breakdowns, and fallback conditions.
- Route receipts capture validation, guardrail, fallback, provider attempt, and reserve/finalize outcomes.
- Gateway tests cover authentication failure, validation failure, budget rejection, no-candidate rejection, provider failure, fallback success, guardrail rejection, and successful completion.
- Follow-on work can add new adapters, strategies, and guardrail stages without expanding the main handler directly.

## Branch And PR Convention

- Branch: `codex/track-03-gateway-routing`
- PR title: `[Track 03] Implement gateway routing and provider adapters`
- Required PR notes:
  - which northbound flow is fully real
  - whether streaming is included or intentionally deferred
  - sample request and response fixtures used in verification
