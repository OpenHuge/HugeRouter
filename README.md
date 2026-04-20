# HugeRouter

Rust backend + TanStack Start frontend monorepo for a protocol-native AI gateway platform. The current implementation baseline is a `pnpm` + `Turbo` workspace for JavaScript packages and a `Cargo` workspace for Rust services/crates, with `Mantine` as the frontend component foundation for the console.

## Document Structure

- [Docs Index](docs/README.md)
- [Architecture Overview](docs/architecture/overview.md)
- [Implementation Contracts](docs/architecture/implementation-contracts.md)
- [Event and Message Contracts](docs/architecture/event-and-message-contracts.md)
- [Persistence Guidance](docs/architecture/persistence-guidance.md)
- [Monorepo and Tech Stack](docs/architecture/monorepo-and-tech-stack.md)
- [Domain Model](docs/architecture/domain-model.md)
- [Protocol IR and Protocol Support (Updated for MCP & WebRTC)](docs/architecture/protocol-ir-and-protocols.md)
- [Routing System](docs/architecture/routing-system.md)
- [Provider Adapter System](docs/architecture/provider-adapter-system.md)
- [Extensibility and Composition](docs/architecture/extensibility-and-composition.md)
- [Open Source Reference Patterns](docs/architecture/open-source-reference-patterns.md)
- [Authentication and Access Control](docs/architecture/auth-and-access-control.md)
- [Metering, Ledger, and Pricing](docs/architecture/metering-ledger-pricing.md)
- [Policy System](docs/architecture/policy-system.md)
- [Observability](docs/architecture/observability.md)
- [Reliability and Performance](docs/architecture/reliability-performance.md)
- [Security and Compliance](docs/architecture/security-and-compliance.md)
- [Frontend Console](docs/architecture/frontend-console.md)
- [Data Storage and Event Flows](docs/architecture/data-storage-and-events.md)
- [Multi-Tenancy and Configuration](docs/architecture/multi-tenancy-and-configuration.md)
- [Services and Crates](docs/architecture/services-and-crates.md)
- [Development, CI/CD, and Testing](docs/runbooks/development-cicd-testing.md)
- [Development Readiness and Bootstrap](docs/runbooks/development-readiness-and-bootstrap.md)
- [Monorepo Boundaries and Bootstrap Contracts](docs/runbooks/monorepo-boundaries-and-bootstrap-contracts.md)
- [Control Plane API](docs/api/control-plane-api.md)
- [Public Gateway API](docs/api/public-gateway-api.md)
- [Roadmap and Delivery Plan](docs/product/roadmap-and-delivery-plan.md)
- [Execution Roadmap Pack](docs/execution/README.md)
- [ADR Index](docs/adr/README.md)
- [Naming Conventions](docs/appendix/naming-conventions.md)

## Scope

This split specification preserves the original engineering intent:

- Rust for backend correctness, performance, and strong domain modeling
- TanStack Start for a typed frontend console
- Mantine for the shared frontend design system and console primitives
- Turbo-managed monorepo workflows for incremental local development and CI
- protocol-native ingress and adapter-based egress (optimized for 2026 multi-modal traffic)
- native support for Model Context Protocol (MCP) and Realtime WebRTC proxying
- immutable usage and ledger design
- first-class routing, semantic caching, policy, security, and observability

## Development Baseline

The repository now includes a working bootstrap baseline:

- `Cargo` workspace for backend crates and services
- `pnpm` workspaces orchestrated by `Turbo` for frontend apps and shared packages
- `TanStack Start` for `apps/console-web`
- `Mantine` for theme, layout, form, feedback, and reusable console components
- `just` as the human-friendly entry point that wraps Rust, `pnpm`, and `turbo` tasks

## Quick Start

Local development:

- `pnpm install`
- `pnpm build`
- `pnpm test`
- `pnpm typecheck`
- `cargo check --workspace`

Convenience commands:

- `just bootstrap`
- `just dev-frontend`
- `just dev-ui`
- `just stack-up`
- `just stack-down`

## Dev Container

The repository includes a compose-based devcontainer in [`.devcontainer/devcontainer.json`](.devcontainer/devcontainer.json).

- The workspace container installs Node `24.15.0`, `pnpm 10.33.0`, Rust `1.95.0`, and uses the official `ghcr.io/devcontainers/features/github-cli:1` feature so `gh` is available the same way it is in GitHub Codespaces.
- `onCreate` installs `@openai/codex` into a user-owned npm global prefix.
- `updateContent` installs workspace dependencies.
- `postCreate` and `postStart` persist Codex config in `.devcontainer/local/codex/config.toml` and GitHub CLI config in `.devcontainer/local/gh`.
- In GitHub Codespaces, `gh` can use the built-in `GITHUB_TOKEN`; an optional recommended `GH_TOKEN` secret is declared for contributors who need fine-grained access to additional repositories. Locally, run `gh auth login` once and the stored auth will persist across rebuilds.
- The devcontainer composes with the local infra stack for PostgreSQL, Redis, NATS, OpenTelemetry Collector, Prometheus, and Grafana.

Recommended entry documents before starting implementation:

- [Implementation Contracts](docs/architecture/implementation-contracts.md)
- [Event and Message Contracts](docs/architecture/event-and-message-contracts.md)
- [Persistence Guidance](docs/architecture/persistence-guidance.md)
- [Monorepo and Tech Stack](docs/architecture/monorepo-and-tech-stack.md)
- [Frontend Console](docs/architecture/frontend-console.md)
- [Development, CI/CD, and Testing](docs/runbooks/development-cicd-testing.md)
- [Development Readiness and Bootstrap](docs/runbooks/development-readiness-and-bootstrap.md)
- [Monorepo Boundaries and Bootstrap Contracts](docs/runbooks/monorepo-boundaries-and-bootstrap-contracts.md)
- [ADR Index](docs/adr/README.md)

## Source

This split Markdown set was derived from the original consolidated specification and reorganized into repository-friendly documents.
