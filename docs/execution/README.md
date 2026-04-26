# Execution Roadmap Pack

[Back to Root README](../../README.md)

This folder turns the architecture specification into an **agent-ready development plan** for the Rust + TanStack Start monorepo, with Turbo for JavaScript workspace orchestration and HeroUI as the console UI foundation.

## What this pack contains

- a multi-phase delivery roadmap
- parallel workstreams that can progress independently
- dependency guidance and critical path notes
- definitions of ready and done
- stream-specific execution documents
- a machine-readable task catalog for agent orchestration

## Recommended reading order

1. [Roadmap Overview](00-roadmap-overview.md)
2. [Parallel Workstreams](01-parallel-workstreams.md)
3. [Program Increments](02-program-increments.md)
4. [Dependency Graph](03-dependency-graph.md)
5. [Agent Operating Model](04-agent-operating-model.md)
6. [Definition of Ready and Done](05-definition-of-ready-and-done.md)
7. [Risk Burndown](06-risk-burndown.md)
8. [Agent Assignment Matrix](07-agent-assignment-matrix.md)
9. Workstream files in [`workstreams/`](workstreams/)
10. Task catalog in [`tasks/`](tasks/)

## Scope boundary

This roadmap assumes the architecture decisions already documented in:

- [Monorepo and Tech Stack](../architecture/monorepo-and-tech-stack.md)
- [Services and Crates](../architecture/services-and-crates.md)
- [Protocol IR and Protocol Support](../architecture/protocol-ir-and-protocols.md)
- [Routing System](../architecture/routing-system.md)
- [Metering, Ledger, and Pricing](../architecture/metering-ledger-pricing.md)
- [Frontend Console](../architecture/frontend-console.md)

## Delivery philosophy

The program is intentionally split into **parallel lanes** so that multiple agents or sub-teams can build the system at the same time without stepping on the same files or waiting on unnecessary dependencies.

The primary mechanism is:

- stabilize interfaces early
- let storage, gateway, frontend, and ops move in parallel
- converge through generated schemas, shared crates, and CI gates
- keep hot-path functionality separate from control-plane and analytics work
