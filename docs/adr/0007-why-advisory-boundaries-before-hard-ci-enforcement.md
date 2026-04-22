# ADR 0007: Why Advisory Boundaries Before Hard CI Enforcement

[Back to ADR Index](README.md)

## Status

Accepted

## Context

The repository is entering implementation as a mixed Rust and JavaScript monorepo. We have already chosen:

- `Turbo` for JavaScript task orchestration
- package configurations for workspace-local overrides
- a composable architecture with shared frontend packages and shared backend contracts

At this stage, repository boundaries matter for two reasons:

1. shared packages such as `packages/ui-kit`, `packages/design-tokens`, and generated clients must not begin importing application-only code
2. feature teams need a visible dependency policy before the repo graph becomes too large to clean up easily

At the same time, the workspace is still in bootstrap. Package names, directory splits, and ownership edges may still shift during the first implementation sprint.

## Decision

We will define repository boundary intent immediately, but enforce it in stages:

1. declare package tags and boundary rules during workspace bootstrap
2. run repository-boundary validation in advisory mode first
3. fix early violations while the package graph is still small
4. promote the boundary check to blocking once the package layout stabilizes

For JavaScript workspaces, this means using per-package `turbo.json` files for tags or narrowly scoped overrides where useful. For Rust workspaces, boundaries remain primarily a matter of crate structure and review discipline.

## Alternatives Considered

### 1. No explicit boundary policy during bootstrap

Rejected because it invites convenient but harmful coupling between `apps/*` and shared packages, which would directly undermine the composability goals of the monorepo.

### 2. Hard-fail CI on boundary rules from day one

Rejected because the workspace is still being scaffolded and would likely generate churn while teams are legitimately moving package responsibilities around.

### 3. Rely only on human review with no automation

Rejected because reviewers are inconsistent at spotting subtle import-graph drift, especially when multiple teams begin working in parallel.

## Consequences

Positive:

- shared packages gain clear ownership intent early
- the repo can scale with less accidental coupling
- CI gains a path to stronger governance without slowing initial scaffolding work

Trade-offs:

- some temporary violations may exist during the advisory phase
- the team must still decide when the graph is stable enough to make the check blocking

## Follow-Up Actions

- add package tags for core JavaScript workspaces during `FND-002`
- add advisory boundary validation during `FND-005`
- promote boundary checks to blocking once the initial package structure is proven in active development
