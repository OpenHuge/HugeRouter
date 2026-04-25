# HugeRouter

Rust backend + TanStack Start frontend monorepo for a trusted AI resource trading platform. The current implementation baseline is a `pnpm` + `Turbo` workspace for JavaScript packages and a `Cargo` workspace for Rust services/crates, with `Mantine` as the frontend component foundation for the console.

## Document Structure

- [Docs Index](docs/README.md)
- [Architecture Overview](docs/architecture/overview.md)
- [Foundation Risk and Trend Analysis](docs/architecture/foundation-risk-and-trend-analysis.md)
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
- [Maintainer Triage](docs/runbooks/maintainer-triage.md)
- [Control Plane API](docs/api/control-plane-api.md)
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
- trusted AI resource listings and seller verification
- replay-backed quality evidence for trial connections
- marketplace auditability and dispute-ready evidence
- immutable usage and ledger design for marketplace evidence
- policy, security, and observability for first-phase trading workflows

## Development Baseline

The repository now includes a working bootstrap baseline:

- `Cargo` workspace for backend crates and services
- `pnpm` workspaces orchestrated by `Turbo` for frontend apps and shared packages
- `TanStack Start` for `apps/console-web`
- `Mantine` for theme, layout, form, feedback, and reusable console components
- `just` as the human-friendly entry point that wraps Rust, `pnpm`, and `turbo` tasks

## Quick Start

Contributor process:

- [Contributing Guide](CONTRIBUTING.md)
- [Support Guide](SUPPORT.md)
- [Security Policy](SECURITY.md)

Local development:

- `pnpm doctor`
- `pnpm install --frozen-lockfile`
- `pnpm build`
- `pnpm test`
- `pnpm typecheck`
- `cargo check --workspace`

Convenience commands:

- `just bootstrap`
- `just doctor`
- `just dev-frontend`
- `just dev-ui`
- `just stack-up`
- `just stack-up-full`
- `just stack-wait`
- `just stack-down`

Quality notes:

- `pnpm js:lint` now runs a repository-wide file-size budget gate before Oxlint so new single-file monoliths are blocked while existing oversized files are tracked with explicit legacy caps in [`scripts/verify-file-size-budgets.mjs`](scripts/verify-file-size-budgets.mjs).
- `pnpm test` runs a workspace test-policy audit before package tests so placeholder scripts are called out explicitly instead of looking like real coverage.
- `pnpm js:lint` also runs a Prettier drift-baseline gate before package lint. This follows Prettier's CI-style `--check` model for newly introduced drift while the repository works down the current formatting backlog tracked in [`scripts/prettier-drift-baseline.json`](scripts/prettier-drift-baseline.json).
- `pnpm js:lint` now runs a repository-wide file-size budget gate before package lint so new single-file monoliths are blocked while existing oversized files are tracked with explicit legacy caps in [`scripts/verify-file-size-budgets.mjs`](scripts/verify-file-size-budgets.mjs).
- Rust quality commands now run with a locked dependency graph so CI fails if `Cargo.lock` would need to change during lint, check, or test.
- Temporary placeholder packages are tracked in [`scripts/workspace-test-policy.json`](scripts/workspace-test-policy.json) until their owning tracks replace them with real tests.
- GitHub Actions quality gates live in [`.github/workflows/quality.yml`](.github/workflows/quality.yml) and run JavaScript lint, typecheck, test, and build jobs plus Rust format, clippy, check, and test jobs on pull requests and `main`.

## Dev Container

The repository includes a compose-based devcontainer in [`.devcontainer/devcontainer.json`](.devcontainer/devcontainer.json).

- The workspace container follows the `OpenHuge/HugeCode` direction of pinning the JavaScript toolchain inside the devcontainer itself, using the official `javascript-node` Bookworm image for Node/npm and installing Rust `1.94.1` plus `gh` in the Dockerfile; `corepack` activates `pnpm 10.33.0` during content updates.
- `onCreate` installs `@openai/codex` into a user-owned npm global prefix.
- `updateContent` runs `pnpm verify:toolchain`, installs workspace dependencies with `--frozen-lockfile`, and warms Cargo dependencies.
- `postCreate` and `postStart` persist Codex config in `.devcontainer/local/codex/config.toml` and GitHub CLI config in `.devcontainer/local/gh`.
- In GitHub Codespaces, `gh` can use the built-in `GITHUB_TOKEN`; an optional recommended `GH_TOKEN` secret is declared for contributors who need fine-grained access to additional repositories. Locally, run `gh auth login` once and the stored auth will persist across rebuilds.
- In GitHub Codespaces, the devcontainer starts the workspace plus PostgreSQL, Redis, and NATS by default so observability sidecars cannot block container creation.
- `just stack-up` starts the core dependency set (`postgres`, `redis`, `nats`).
- `just stack-up-full` adds the observability profile (`otel-collector`, `prometheus`, `grafana`) on top of the core stack.
- `just stack-up-observability` is available when you only want the observability sidecars.
- `just stack-wait` waits for the selected stack mode to report healthy containers before you boot services against it.
- The runtime profile also starts the route-health, audit, and notification workers. `EDGE_PROBE_PROBE_MODE=cheap_health` is the default non-billable health check mode, while `billable_synthetic` is reserved for explicit paid synthetic traffic.
- The NATS service is pinned to `nats:2.12.7-alpine3.22` because the shared `nats:2.12.7` tag resolves to a `scratch` variant, which does not include `/bin/sh` or `wget` and therefore cannot satisfy the configured health check.
- The PostgreSQL service is pinned to `postgres:18.3-bookworm`. This is a Debian-based image, not Alpine, and PostgreSQL 18+ expects the persistent volume to target `/var/lib/postgresql` rather than `/var/lib/postgresql/data`.

Recommended entry documents before starting implementation:

- [Implementation Contracts](docs/architecture/implementation-contracts.md)
- [Event and Message Contracts](docs/architecture/event-and-message-contracts.md)
- [Persistence Guidance](docs/architecture/persistence-guidance.md)
- [Monorepo and Tech Stack](docs/architecture/monorepo-and-tech-stack.md)
- [Frontend Console](docs/architecture/frontend-console.md)
- [Development, CI/CD, and Testing](docs/runbooks/development-cicd-testing.md)
- [Development Readiness and Bootstrap](docs/runbooks/development-readiness-and-bootstrap.md)
- [Monorepo Boundaries and Bootstrap Contracts](docs/runbooks/monorepo-boundaries-and-bootstrap-contracts.md)
- [Maintainer Triage](docs/runbooks/maintainer-triage.md)
- [ADR Index](docs/adr/README.md)

## Source

This split Markdown set was derived from the original consolidated specification and reorganized into repository-friendly documents.
