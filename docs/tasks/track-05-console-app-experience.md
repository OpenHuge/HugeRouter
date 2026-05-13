# Track 05 - Console App Experience

## Mission

Turn `apps/console-web` from a data-backed gateway console into an operator-ready control plane for cost governance, route reliability, model capability selection, guardrails, auditability, and enterprise/channel administration.

## Current Baseline

- The app has a TanStack Start router, HeroUI providers, authenticated shell, login and callback flow, tenant routes, admin tenant routes, provider resources, route policies, config snapshots, API keys, route diagnostics, route receipts, usage, billing, and merchant relay evaluation pages.
- Data access goes through `@huge-router/ts-api-client` and shared schemas, with route loaders and control-plane fetch test fixtures.
- Auth screens exercise email and OAuth-oriented flows against backend-owned session contracts, with provider-disabled and callback states covered in tests.
- Usage and billing screens are data-backed, but they still present basic summaries rather than the richer financial/operator views needed for cost governance.
- Route receipt and diagnostics screens expose important signals, but they do not yet show the full fallback chain, reserve/finalize lifecycle, live health source, guardrail findings, or redaction policy.
- The next console gaps are pricing/budget authoring, model capability matrix, strategy-aware route policy authoring, guardrail policy authoring, audit/payload access review, channel settlement, and role-specific dashboards.

## Owned Paths

- `apps/console-web/*`

## Do Not Edit

- `packages/ui-kit`
- `packages/test-utils`
- `packages/ts-api-client`
- `packages/ts-shared-schema`
- backend services and crates

If a needed UI primitive or client contract is missing, consume the public surface from Tracks `01` and `06` after they merge instead of defining a private duplicate here.

## Deliverables

1. Pricing catalog, budget policy, and cost attribution surfaces.
2. Route strategy, fallback, circuit breaker, and live health diagnostics surfaces.
3. Model capability matrix and downgrade guidance for route authoring.
4. Guardrail, redaction, and sealed payload access policy surfaces.
5. Role-specific dashboards for engineering, finance, operations, security, and channel operators.
6. A real login UX for email, GitHub, Google, and WeChat backed by Track `02` auth APIs.
7. A bilingual console baseline for Simplified Chinese and English, including locale selection and shared formatting behavior.
8. Frontend tests that prove important workflows, errors, optimistic states, and access controls.

## Ordered Plan

1. Build cost governance screens.
   - Add pricing catalog list/edit flows once Track `02` exposes managed catalogs.
   - Add budget policy authoring by tenant, project, key, user/app, day, and month.
   - Add spend, provider cost, billable price, gross margin, high-cost requests, and anomaly views.
2. Upgrade route authoring and diagnostics.
   - Add strategy controls for cost, latency, quality, availability, region, and customer tier.
   - Show live health/rate-limit/circuit-breaker inputs and the full fallback chain.
   - Surface reserve/finalize budget decisions next to route receipts.
3. Add model capability matrix.
   - Show per-provider and per-model support for streaming, tools, JSON schema, long context, vision, images, audio, embeddings, rerank, batch, and realtime.
   - Use the matrix inside route policy forms to prevent incompatible selections.
4. Add guardrail and redaction UX.
   - Configure PII redaction, prompt-injection checks, output validation, content safety, retention, and sealed payload capture.
   - Provide audit views for policy changes and payload access.
5. Preserve login, session, and locale foundations.
   - Move request and session context setup behind one explicit boundary.
   - Keep the auth model intentionally simple if Track `02` has not yet delivered full identity support, but do not invent a parallel auth contract in app code.
   - Wire email login start and completion UI.
   - Wire GitHub, Google, and WeChat sign-in buttons to backend-owned start and callback paths.
   - Add callback completion, cancellation, and failure states.
   - Fetch the current HugeRouter session and tenant membership before rendering authenticated routes.
   - Support provider-disabled and tenant-access-denied states without dropping into generic transport errors.
   - Move user-facing copy out of route-local literals and into a translation boundary that can serve `zh-CN` and `en`.
   - Support locale selection on `/login` and preserve it through the authenticated shell.
   - Format dates, times, numbers, and currency through shared locale-aware helpers instead of inline formatting.
6. Make state transitions explicit.
   - Add empty, loading, optimistic, and failure states.
   - Avoid rendering raw transport errors directly into the UI.
7. Keep app code inside app ownership.
   - Consume `ui-kit`, `ts-api-client`, and `ts-shared-schema` as published packages.
   - Do not duplicate shared components or schemas in `apps/console-web`.

## Required Tests

- Route and component tests for pricing, budget, route policy, route diagnostics, guardrail, audit, usage, billing, and admin surfaces.
- Tests for loading, empty, success, and error states using deterministic client mocks or request handlers.
- Tests for navigation behavior and route guards where auth/session logic changes.
- Accessibility checks for key screens and forms where practical.
- Tests that cover all supported login entry points: email, GitHub, Google, and WeChat.
- Tests for callback success, callback failure, logout, expired session handling, and tenant-selection or tenant-denied states where applicable.
- Tests for locale switching and fallback behavior on `/login` and at least one authenticated route in both `zh-CN` and `en`.

## Definition Of Done

- Operator workflows for pricing, budgets, route strategies, diagnostics, capabilities, guardrails, and audit are driven by typed client calls.
- Existing login and authenticated route entry continue to use backend-owned session contracts.
- Tests cover the major state transitions of the new operator screens.
- Placeholder bootstrap data is removed from page implementations.
- `/login` and authenticated route entry no longer stop at presentation-only auth placeholders.
- The login flow and shared shell render in both `zh-CN` and `en` without route-local hardcoded copy.
- The app consumes shared packages instead of redefining contracts or primitives locally.
- Future UI work can add features by extending established data and state patterns.

## Branch And PR Convention

- Branch: `codex/track-05-console-app`
- PR title: `[Track 05] Implement console app data flows`
- Required PR notes:
  - which screens are now data-backed
  - which auth entry points and callback states are covered
  - what was mocked versus served by real APIs during verification
  - screenshots for new UI states
