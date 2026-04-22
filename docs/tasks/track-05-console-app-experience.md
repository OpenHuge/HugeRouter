# Track 05 - Console App Experience

## Mission

Turn `apps/console-web` from a polished shell with placeholder content into a data-driven console that exercises the real control-plane surface and shared frontend contracts.

## Current Baseline

- The app already has a TanStack Start router, Mantine providers, shared navigation, login page, overview page, admin layout, and admin tenants page.
- The current request context in `src/start.ts` is placeholder-only.
- `app.overview` and `admin.tenants` are still wired to placeholder data and copy.
- There is only one meaningful UI test today: `src/test/login-page.test.tsx`.
- The login page now signals email, GitHub, Google, and WeChat sign-in options, but those buttons are still presentation-only and do not exercise real callback, session, or tenant-resolution logic.

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

1. Real route-level data loading using the typed client from Track `01`.
2. Auth/session handling appropriate to the current platform maturity, without leaking bootstrap placeholders into page code.
3. Data-backed overview, tenant, provider, and route-management surfaces for the first control-plane slice.
4. Loading, empty, error, and success states that use shared frontend patterns.
5. Frontend tests that prove behavior at the route and component level.
6. A real login UX for email, GitHub, Google, and WeChat backed by Track `02` auth APIs.

## Ordered Plan

1. Replace placeholder app bootstrap.
   - Move request and session context setup behind one explicit boundary.
   - Keep the auth model intentionally simple if Track `02` has not yet delivered full identity support, but do not invent a parallel auth contract in app code.
2. Wire real data access.
   - Replace placeholder arrays and mock project summaries with typed client calls.
   - Use route loaders or query hooks consistently instead of page-local ad hoc fetching.
3. Implement the real login and session flow.
   - Wire email login start and completion UI.
   - Wire GitHub, Google, and WeChat sign-in buttons to backend-owned start and callback paths.
   - Add callback completion, cancellation, and failure states.
   - Fetch the current HugeRouter session and tenant membership before rendering authenticated routes.
   - Support provider-disabled and tenant-access-denied states without dropping into generic transport errors.
4. Build the first real feature surfaces.
   - Expand overview beyond static cards.
   - Turn admin tenancy into a real list and detail flow.
   - Add providers and routes surfaces only after the necessary APIs exist.
5. Make state transitions explicit.
   - Add empty, loading, optimistic, and failure states.
   - Avoid rendering raw transport errors directly into the UI.
6. Keep app code inside app ownership.
   - Consume `ui-kit`, `ts-api-client`, and `ts-shared-schema` as published packages.
   - Do not duplicate shared components or schemas in `apps/console-web`.

## Required Tests

- Route and component tests for the login flow, overview, and admin tenancy surfaces.
- Tests for loading, empty, success, and error states using deterministic client mocks or request handlers.
- Tests for navigation behavior and route guards where auth/session logic changes.
- Accessibility checks for key screens and forms where practical.
- Tests that cover all supported login entry points: email, GitHub, Google, and WeChat.
- Tests for callback success, callback failure, logout, expired session handling, and tenant-selection or tenant-denied states where applicable.

## Definition Of Done

- The main console pages are driven by typed client calls rather than placeholder literals.
- Placeholder bootstrap data is removed from page implementations.
- `/login` and authenticated route entry no longer stop at presentation-only auth placeholders.
- Tests cover the major state transitions of the first real screens.
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
