# Parallel Development Tracks

This directory is the historical execution-ready version of the earlier AI gateway plan. The Phase 1 main branch has since been narrowed to the trusted AI resource trading platform; gateway runtime work should continue only from the standalone branch described in [`../product/route-gateway-standalone-branch.md`](../product/route-gateway-standalone-branch.md).

## Repository Snapshot

- The TypeScript workspace is real and data-backed. `apps/console-web` has authenticated routes, login and callback screens, tenant/admin areas, provider/resource management, route policy management, config snapshots, API keys, route receipts, usage, billing, and merchant relay evaluation surfaces. `packages/ts-api-client` and `packages/ts-shared-schema` now contain meaningful contract-backed code and tests rather than pure mock placeholders.
- The Rust workspace now keeps the control-plane, workers, domain, provider metadata, pricing, billing, auth, and marketplace surfaces in main. Gateway runtime services were removed from main to reduce Phase 1 scope.
- `control-plane-api` now exposes persisted tenant, project, provider resource, route policy, config snapshot, API key, usage, billing, route receipt, auth, and merchant endpoints with memory and Postgres-backed paths.
- `ledger-worker` and `route-receipt-worker` process real event payloads into ledger/projection and route diagnostic tables. `audit-worker`, `notification-worker`, `routing-worker`, and parts of `edge-probe` still need stronger operational behavior beyond the first runtime scaffolding.
- The workspace targets Rust `1.94.1`, Node `24.15.0`, and pnpm `10.33.0`. GitHub Actions quality and release workflows are present.
- Test coverage is meaningful in several core paths but still uneven. The next phase should increase integration coverage around pricing, budget admission, route health, fallback diagnostics, and console workflows.
- The product direction for Phase 1 is now a trusted AI resource trading platform, not a cheap-key forwarding panel or general-purpose AI relay. Gateway/routing reliability work belongs on the standalone feature branch.

## Global Rules For Every Agent

1. Stay inside the file ownership listed in your track document. If you need a change in another track's area, open a follow-up PR or coordinate after that track merges.
2. Do not keep placeholder behavior once you touch a path. Replace it with real implementation or remove it.
3. Every PR must include rigorous automated tests for the behavior it adds or changes.
4. Prefer small, mergeable PRs inside the track rather than one giant branch that tries to solve the whole stream.
5. If a track depends on another track's merged contract, rebase onto `main` after that dependency lands instead of inventing a parallel contract.
6. Distinguish shipped, bootstrap, simulated, and planned behavior in docs and PR notes. Do not describe sample projections, default pricing, or replay simulations as production-complete capabilities.
7. Treat provider provenance as a product and safety boundary. Routes backed by unofficial or opaque upstream capacity must be visible, diagnosable, and lower trust than official or customer-owned capacity.

## Required Engineering Bar

- Code must be production-oriented, not demo-oriented.
- Unit tests are mandatory for domain logic, validators, serializers, adapters, hooks, and component state logic.
- Integration tests are mandatory when a PR touches HTTP boundaries, persistence, worker/event handling, or generated contracts.
- PRs must not weaken existing checks. If local verification is blocked, document the exact blocker in the PR body and keep the unresolved surface narrow.
- New behavior must be discoverable from docs, tests, or typed interfaces without requiring tribal knowledge.

## Auth Work Split

- Track `01` owns shared identity, session, membership, and auth-provider contracts consumed by backend and frontend code.
- Track `02` owns control-plane auth APIs, provider configuration, session issuance, account linking, membership resolution, and auth audit events.
- Track `05` owns the `/login` experience, callback and failure handling, route guards, and tenant-aware session UX in `apps/console-web`.
- If a change mixes contract, backend, and UI auth work, land it as multiple PRs in that order instead of inventing private stopgap models.

## Branch And PR Convention

- Branch name: `codex/track-XX-short-scope`
- PR title: `[Track XX] Short outcome statement`
- Base branch: `main`
- PR checklist:
  - summary of shipped behavior
  - exact verification commands and results
  - follow-up items intentionally left out of scope
  - screenshots or sample payloads when UI or API behavior changes

## Track Matrix

| Track | Focus                                                        | Primary Ownership                                                                                                                          | Depends On                 |
| ----- | ------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------- |
| `00`  | Foundation, toolchain, CI, and quality gates                 | root configs, `turbo.json`, `justfile`, `.devcontainer`, `.github`, `infra/*`                                                              | none                       |
| `01`  | Domain contracts, schema pipeline, and shared auth contracts | `crates/core-domain`, `crates/protocol-ir`, `schemas/*`, `packages/ts-api-client`, `packages/ts-shared-schema`                             | `00` recommended           |
| `02`  | Control plane, config activation, and auth backend flows     | `services/control-plane-api`, new backend support crates owned by this track                                                               | `01`                       |
| `03`  | Archived gateway, routing, and provider adapter plan         | Standalone route gateway branch only                                                                                                       | `01`, `02` partially       |
| `04`  | Metering, ledger, and operational workers                    | `services/ledger-worker`, `services/audit-worker`, `services/notification-worker`, `services/routing-worker`, `crates/runtime-composition` | `01`, `03`                 |
| `05`  | Console app experience and auth UX                           | `apps/console-web`                                                                                                                         | `01`, `02`, `06` partially |
| `06`  | UI system, Storybook, and frontend quality                   | `packages/ui-kit`, `packages/test-utils`, `apps/storybook`                                                                                 | none                       |

## Next Implementation Loop

The next PR sequence should move the main product toward a trusted AI resource trading platform:

1. Phase 1: documentation and backlog calibration against current `main`.
2. Phase 2: cost and budget governance, including persistent pricing catalog, request pre-admission estimates, reserve accounting, and richer margin reporting.
3. Phase 3: seller verification, listing governance, escrow/accounting, resource quality evidence, and replay-backed trust signals.
4. Phase 4: model/resource capability matrix for marketplace discovery and compatibility claims.
5. Phase 5: redaction-first observability and audit trails for quality evidence and marketplace operations.
6. Phase 6: enterprise and channel governance, including RBAC depth, SSO hardening, tenant data-retention policy, channel pricing, and role-specific dashboards.

## Recommended Merge Order

1. Keep Track `01` contract changes ahead of backend and frontend consumers.
2. Land cost/budget changes through Tracks `01`, `02`, `03`, `04`, and `05` in that order when schema, admission, worker, and console work all change.
3. Keep route gateway runtime changes out of main unless they are explicit integration contracts consumed by the marketplace.
4. Land guardrails through explicit contracts first, then control-plane/worker enforcement, then console/audit surfaces.
5. Keep UI-system changes in Track `06` only when multiple console surfaces need the primitive.

## Track Documents

- [Track 00 - Foundation And Quality](./track-00-foundation-and-quality.md)
- [Track 01 - Domain Contracts And Schema Pipeline](./track-01-domain-contracts-and-schema-pipeline.md)
- [Track 02 - Control Plane And Config Activation](./track-02-control-plane-and-config-activation.md)
- [Track 03 - Gateway, Routing, And Provider Adapters](./track-03-gateway-routing-and-provider-adapters.md)
- [Track 04 - Metering, Ledger, And Workers](./track-04-metering-ledger-and-workers.md)
- [Track 05 - Console App Experience](./track-05-console-app-experience.md)
- [Track 06 - UI System, Storybook, And Frontend Quality](./track-06-ui-system-storybook-and-frontend-quality.md)
