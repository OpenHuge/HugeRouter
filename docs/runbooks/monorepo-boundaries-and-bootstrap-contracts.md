# Monorepo Boundaries and Bootstrap Contracts

[Back to Docs Index](../README.md)

This runbook defines the minimum repository governance needed to keep the monorepo composable as implementation begins.

## 1. Purpose

The project is now ready to move from architecture design into implementation, but the monorepo needs explicit dependency and ownership rules so early convenience does not become long-term coupling.

This document freezes:

- package and crate boundary expectations
- root file responsibilities
- initial package tags and dependency rules
- the enforcement rollout for repository boundaries

## 2. Decision Snapshot

As of **April 20, 2026**, the working baseline is:

- use `Turbo` as the JavaScript task orchestrator
- use package-level `turbo.json` files only when a workspace needs local overrides or tags
- declare package tags early so dependency intent is visible before the repo becomes large
- run boundary validation in advisory mode first, then promote it to blocking once the graph stabilizes

This matches the current state of Turborepo, where package configurations are mature enough for targeted overrides, while Boundaries remains useful but still best adopted pragmatically during bootstrap.

## 3. Root Ownership Contracts

These root files are not interchangeable. Each exists to control one layer of the repository:

- `Cargo.toml`: Rust workspace membership, shared dependencies, and backend build graph
- `package.json`: workspace-level JavaScript scripts, `packageManager`, and shared dev tooling entry points
- `pnpm-workspace.yaml`: JavaScript workspace membership only
- `turbo.json`: repo-level task graph, cache policy, filters, and optional boundary rules
- `justfile`: human-facing command surface for common multi-tool workflows
- `rust-toolchain.toml`: pinned Rust toolchain and component baseline

Implementation rule:

- do not hide cross-workspace orchestration logic inside individual package scripts if it belongs at the repo level
- do not push application-specific behavior into root scripts when a package-local script is sufficient

## 4. JavaScript Workspace Classes

The initial JavaScript workspace should use the following conceptual classes:

- `app`: deployable user-facing applications such as `apps/console-web`
- `ui-public`: reusable UI packages safe for broad frontend consumption
- `ui-internal`: shell or admin-specific UI packages that should not become public dependencies automatically
- `frontend-contract`: generated clients, shared schemas, and design tokens
- `tooling`: lint, TypeScript, testing, or build configuration packages
- `docs`: optional documentation-only packages or content workspaces

Recommended first mapping:

- `apps/console-web`: `app`
- `apps/storybook`: `app`
- `packages/ui-kit`: `ui-public`
- `packages/design-tokens`: `frontend-contract`
- `packages/ts-api-client`: `frontend-contract`
- `packages/ts-shared-schema`: `frontend-contract`
- `packages/typescript-config`: `tooling`
- `packages/test-utils`: `tooling`

## 5. JavaScript Dependency Rules

Day-0 dependency policy should be:

- `app` may depend on `ui-public`, `ui-internal`, `frontend-contract`, and `tooling`
- `ui-public` may depend on `frontend-contract` and `tooling`
- `ui-internal` may depend on `ui-public`, `frontend-contract`, and `tooling`
- `frontend-contract` may depend on `tooling`, but not on `app`, `ui-internal`, or feature code
- `tooling` must not depend on `app`
- no package in `packages/*` may import from `apps/*`

Practical interpretation:

- `packages/ui-kit` can use HeroUI and shared schemas, but it cannot import app routes, auth bootstrapping, or page-specific feature modules
- `packages/design-tokens` must remain framework-light and free of app state or transport concerns
- generated clients and schemas should flow outward into apps, never the reverse

## 6. Rust Workspace Boundary Rules

Rust boundaries are enforced through workspace structure, review discipline, and crate API design rather than Turbo.

The initial crate categories should be:

- `domain`: core business types and invariants
- `protocol`: parsing, IR, and serialization logic
- `plugin-contract`: stable extension interfaces and manifests
- `provider-trait`: provider-facing ports and adapter contracts
- `provider-impl`: upstream implementations
- `platform`: telemetry, config, storage, queue, cache, and auth utilities
- `service`: composition roots and executable binaries

Rust dependency policy:

- `domain` crates must not depend on `service` crates
- `plugin-contract` crates must not depend on provider implementations
- `provider-impl` crates may depend on contracts, protocol crates, and platform crates, but not on service crates
- `service` crates own wiring and runtime assembly; they must not be imported by reusable library crates

## 7. Bootstrap File Contracts

Before feature work begins, the first implementation slice should produce these minimum contracts:

### 7.1 Root `package.json`

Must define:

- `packageManager`
- root scripts for `dev`, `build`, `lint`, `typecheck`, `test`, and `generate`
- no hidden install-time side effects

### 7.2 Root `turbo.json`

Must define:

- repo-wide task graph
- explicit `outputs` for every cacheable task
- persistent non-cacheable tasks for `dev` and `storybook`
- optional boundary rules that can be checked in CI

### 7.3 Package-Level `turbo.json`

Use only where needed for:

- tags
- framework-specific task overrides
- package-local `dependsOn` or cache tuning that should not affect unrelated workspaces

### 7.4 `apps/console-web`

Must contain:

- TanStack Start app bootstrap
- `src/start.ts` for global middleware and Start-level configuration
- authenticated and unauthenticated route shells
- consumption of `packages/design-tokens` and `packages/ui-kit`

### 7.5 Shared Frontend Packages

Must separate concerns cleanly:

- `packages/design-tokens`: theme, semantic tokens, density, spacing, typography
- `packages/ui-kit`: shell primitives, feedback surfaces, form wrappers, layout components
- `packages/ts-api-client`: generated client only
- `packages/ts-shared-schema`: shared browser-safe schemas only

## 8. Boundary Enforcement Rollout

Do not turn every rule into a hard blocker on day one.

Recommended rollout:

1. declare package tags and dependency rules during workspace bootstrap
2. add `turbo boundaries` or equivalent validation as a non-blocking CI signal
3. fix obvious violations while the graph is still small
4. promote the boundary job to blocking once the initial package layout and import graph have stabilized

Blocking too early creates churn while packages are still moving. Waiting too long allows accidental app-to-package coupling to harden.

## 9. Definition of Ready for Feature Teams

Frontend feature work may begin when all of the following are true:

- root workspace files exist and are pinned
- `apps/console-web` boots with HeroUI and TanStack Start middleware
- core shared packages have declared ownership and dependency intent
- package tags are present for the initial JavaScript workspaces
- CI can detect boundary drift, even if the first phase is advisory

At that point the repository is structured enough to support parallel implementation without reopening monorepo governance on every pull request.
