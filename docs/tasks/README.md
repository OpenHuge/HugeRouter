# Parallel Development Tracks

This directory is the execution-ready version of the project plan. The existing material under `docs/execution` and `docs/product` is still useful for roadmap context, but agents should treat the files in `docs/tasks` as the active implementation backlog because they reflect the repository's actual state on 2026-04-22.

## Repository Snapshot

- The TypeScript workspace is real and usable, but still shallow. `apps/console-web` has a working TanStack Start shell, login page, overview page, admin tenants page, and one meaningful Vitest test. `packages/ui-kit`, `packages/ts-api-client`, and `packages/ts-shared-schema` exist, but they are still minimal.
- The Rust workspace has meaningful domain and protocol scaffolding in `crates/core-domain` and `crates/protocol-ir`, plus a bootstrap `gateway-api` flow that returns an OpenAI-shaped placeholder response. Most other Rust services are startup placeholders only.
- The workspace now targets Rust `1.94.1`, matching the current local toolchain and avoiding the previous bootstrap failure caused by a higher pinned version.
- Test coverage is not yet representative. Several packages still use no-op test scripts, and there are no backend integration tests yet.

## Global Rules For Every Agent

1. Stay inside the file ownership listed in your track document. If you need a change in another track's area, open a follow-up PR or coordinate after that track merges.
2. Do not keep placeholder behavior once you touch a path. Replace it with real implementation or remove it.
3. Every PR must include rigorous automated tests for the behavior it adds or changes.
4. Prefer small, mergeable PRs inside the track rather than one giant branch that tries to solve the whole stream.
5. If a track depends on another track's merged contract, rebase onto `main` after that dependency lands instead of inventing a parallel contract.

## Required Engineering Bar

- Code must be production-oriented, not demo-oriented.
- Unit tests are mandatory for domain logic, validators, serializers, adapters, hooks, and component state logic.
- Integration tests are mandatory when a PR touches HTTP boundaries, persistence, worker/event handling, or generated contracts.
- PRs must not weaken existing checks. If local verification is blocked, document the exact blocker in the PR body and keep the unresolved surface narrow.
- New behavior must be discoverable from docs, tests, or typed interfaces without requiring tribal knowledge.

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

| Track | Focus | Primary Ownership | Depends On |
|---|---|---|---|
| `00` | Foundation, toolchain, CI, and quality gates | root configs, `turbo.json`, `justfile`, `.devcontainer`, `.github`, `infra/*` | none |
| `01` | Domain contracts and schema pipeline | `crates/core-domain`, `crates/protocol-ir`, `schemas/*`, `packages/ts-api-client`, `packages/ts-shared-schema` | `00` recommended |
| `02` | Control plane and config activation | `services/control-plane-api`, new backend support crates owned by this track | `01` |
| `03` | Gateway, routing, and provider adapters | `services/gateway-api`, `crates/provider-traits`, new gateway/provider crates | `01`, `02` partially |
| `04` | Metering, ledger, and operational workers | `services/ledger-worker`, `services/audit-worker`, `services/notification-worker`, `services/routing-worker`, `crates/runtime-composition` | `01`, `03` |
| `05` | Console app experience | `apps/console-web` | `01`, `02`, `06` partially |
| `06` | UI system, Storybook, and frontend quality | `packages/ui-kit`, `packages/test-utils`, `apps/storybook` | none |

## Recommended Merge Order

1. Track `00` should land first because it removes toolchain and CI ambiguity.
2. Track `01` should land next because it stabilizes contracts other tracks consume.
3. Tracks `02`, `03`, and `06` can move in parallel after `01` is underway, but should rebase frequently.
4. Track `05` can start immediately against placeholders, but should hold final integration until `02` and `06` have merged the contracts and shared components it depends on.
5. Track `04` should begin after `03` has established real event emission and routing outputs.

## Track Documents

- [Track 00 - Foundation And Quality](./track-00-foundation-and-quality.md)
- [Track 01 - Domain Contracts And Schema Pipeline](./track-01-domain-contracts-and-schema-pipeline.md)
- [Track 02 - Control Plane And Config Activation](./track-02-control-plane-and-config-activation.md)
- [Track 03 - Gateway, Routing, And Provider Adapters](./track-03-gateway-routing-and-provider-adapters.md)
- [Track 04 - Metering, Ledger, And Workers](./track-04-metering-ledger-and-workers.md)
- [Track 05 - Console App Experience](./track-05-console-app-experience.md)
- [Track 06 - UI System, Storybook, And Frontend Quality](./track-06-ui-system-storybook-and-frontend-quality.md)
