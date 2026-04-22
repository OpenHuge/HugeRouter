# ADR 0003: Why Turborepo for Workspace Orchestration

[Back to ADR Index](README.md)

## Status

Accepted

## Context

The repository is a mixed Rust and TypeScript monorepo. We need a JavaScript workspace orchestrator that can:

- run application and package tasks incrementally
- support fast local iteration for `apps/console-web` and shared frontend packages
- keep CI selective and cache-aware
- work cleanly with `pnpm` workspaces, Storybook, generated clients, and future docs apps
- stay simple enough that the team can understand and maintain it early in the project

We already have `Cargo` as the Rust-native build and dependency graph. The missing decision is the orchestration layer for JavaScript and repo-level frontend workflows.

## Decision

Use `Turbo` as the required JavaScript workspace orchestrator.

`Turbo` will coordinate repo-level frontend and shared-package tasks such as:

- `dev`
- `build`
- `lint`
- `typecheck`
- `test`
- `storybook`
- `generate`

`Cargo` remains authoritative for Rust compilation, testing, and dependency resolution. `just` remains the top-level developer entry point that can wrap both `cargo` and `turbo` commands.

## Alternatives Considered

### `Nx`

`Nx` offers strong graph tooling and broad ecosystem support, but it introduces more platform surface area than we need right now. For the current repository scope, its additional abstraction does not clearly outweigh the operational simplicity of `Turbo`.

### Plain `pnpm` scripts without a task orchestrator

Plain workspace scripts are viable for very small repos, but they do not give us the same cache model, filtered execution, or long-term CI ergonomics. We would likely reintroduce orchestration later after the workspace had already grown around ad hoc command patterns.

### Custom scripting through `just` only

`just` is useful as a command facade, but it is not a replacement for task graph orchestration or package-aware caching. Using it alone would shift too much responsibility into handwritten scripts.

## Consequences

Positive:

- clearer local and CI workflows for frontend and shared packages
- fast filtered execution for feature work
- a simple mental model for package task orchestration
- easier adoption of Storybook and generated-client pipelines

Tradeoffs:

- another root configuration file to maintain
- developers must learn `Turbo` filter and cache behavior
- repo task naming should stay disciplined to avoid pipeline drift

## Follow-up Actions

- create `turbo.json` with the standard task pipeline
- align package scripts around shared task names
- document `just` to `turbo` command mappings in the development runbook
- add CI caching and filtered execution based on Turbo pipelines
