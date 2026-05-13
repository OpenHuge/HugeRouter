# Track 06 - UI System, Storybook, And Frontend Quality

## Mission

Build the shared frontend layer that the console can rely on: reusable UI primitives, Storybook coverage, and real tests for the frontend packages that are currently no-ops.

## Current Baseline

- `packages/ui-kit` currently exposes a small but real shell layer: `AppShellFrame`, `PageHeader`, and `EmptyState`.
- `packages/test-utils` provides shared React testing helpers and now runs Vitest rather than a no-op script.
- `apps/storybook` is configured and contains a single story for shell components.
- `ui-kit`, `test-utils`, `ts-api-client`, and `ts-shared-schema` have real test commands. `design-tokens` still has an explicit no-test placeholder and should stay tracked by the workspace test policy until it gains assertions.

## Owned Paths

- `packages/ui-kit/*`
- `packages/test-utils/*`
- `apps/storybook/*`

## Do Not Edit

- `apps/console-web`
- `packages/ts-api-client`
- `packages/ts-shared-schema`
- backend services and crates

This track exists so the app track can consume a stable shared layer instead of inventing one inside the app.

## Deliverables

1. A richer shared component set for dense operator tables, filters, forms, loading states, error states, data display, audit timelines, and status badges.
2. Storybook stories that document shared component behavior and edge states.
3. Real package-level tests for `ui-kit` and `test-utils`.
4. Frontend verification patterns that Track `05` can reuse instead of redefining locally.
5. Removal of no-op package tests in this ownership area.
6. Shared locale-aware primitives and examples that let `apps/console-web` support Simplified Chinese and English without app-local duplication.

## Ordered Plan

1. Expand `ui-kit` intentionally.
   - Add only components that solve repeated console problems.
   - Keep app-specific business logic out of the package.
   - Prioritize dense operational surfaces over marketing-style cards.
   - Add shared locale-aware primitives only where repeated formatting or translated affordances belong in the reusable layer.
2. Strengthen test utilities.
   - Make provider wrappers, query helpers, and common assertions reusable.
   - Keep the API ergonomic enough that app tests naturally adopt it.
3. Grow Storybook coverage.
   - Add stories for normal, loading, empty, and error states.
   - Include fixtures that match real schema shapes from Track `01`.
   - Cover compact table/form states used by pricing, budgets, route diagnostics, guardrails, and audit pages.
   - Include at least one `zh-CN` and one `en` story variant for components that render user-facing text or billing values.
4. Replace no-op tests.
   - Add meaningful package tests for rendering, accessibility basics, and interactions where applicable.
   - Ensure the test command fails on regressions instead of printing a placeholder message.
5. Publish usage patterns.
   - Document how `console-web` should consume the new primitives and helpers.
   - Keep the public exports stable and easy to scan.

## Required Tests

- Component tests for shared UI primitives.
- Tests for provider wrappers and helper utilities in `packages/test-utils`.
- Storybook smoke or interaction coverage for key shared states.
- Verification that package exports remain type-safe and consumable by `apps/console-web`.
- Verification that locale-sensitive shared components render correctly for both `zh-CN` and `en`.

## Definition Of Done

- `ui-kit` provides enough reusable surface that Track `05` does not need to create parallel app-local primitives.
- Storybook demonstrates the important shared operator states instead of a single shell snapshot.
- Package test scripts in this track run real assertions.
- Shared test utilities reduce boilerplate in downstream app tests.
- Shared primitives and stories demonstrate the supported `zh-CN` and `en` locale behaviors.
- Public exports are intentional, documented by stories or tests, and stable enough for follow-on work.

## Branch And PR Convention

- Branch: `codex/track-06-ui-system`
- PR title: `[Track 06] Expand UI system and frontend quality checks`
- Required PR notes:
  - which shared primitives were added
  - which no-op tests were replaced
  - links or screenshots for new Storybook coverage
