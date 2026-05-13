# Contributing

HugeRouter is organized as a Rust backend plus TanStack Start monorepo. Contributions should keep the control plane, gateway data plane, protocol contracts, and console UI independently reviewable.

## Start Here

1. Read [Development Readiness and Bootstrap](docs/runbooks/development-readiness-and-bootstrap.md).
2. Install the pinned toolchain:

   ```bash
   corepack enable
   corepack prepare pnpm@10.33.0 --activate
   pnpm install --frozen-lockfile
   ```

3. Verify your environment:

   ```bash
   pnpm verify:toolchain
   pnpm rust:check
   ```

## Branch Scope

Prefer one coherent product or platform slice per pull request. Good examples:

- one protocol mapping or adapter change
- one control-plane API flow plus generated contract updates
- one console workflow with focused tests
- one runtime/CI quality gate improvement

Avoid mixing unrelated refactors, formatting churn, generated contract changes, and feature work in the same PR.

## Local Checks

Run the smallest useful checks while developing, then run the broader gates before asking for review:

```bash
pnpm lint
pnpm typecheck
pnpm test
pnpm build
pnpm rust:fmt:check
pnpm rust:lint
pnpm rust:test
```

When a change touches Docker Compose or runtime scripts, also run:

```bash
docker compose -f infra/docker/compose.yaml --profile runtime --profile observability config -q
bash -n infra/scripts/*.sh infra/scripts/lib/*.sh
```

When a change touches protocol contracts, regenerate and verify:

```bash
pnpm generate
cargo test -p protocol-ir --locked checked_in_artifacts_match_generated_contracts -- --exact
```

## Architecture Rules

- Keep data-plane request execution independent from live control-plane availability wherever possible.
- Treat route receipts, usage events, audit events, and replay capsules as operational contracts, not logging conveniences.
- Preserve typed schemas at service, protocol, and package boundaries.
- Keep provider-specific behavior visible enough for diagnostics while mapping common behavior into canonical domain types.
- Add shared abstractions only when they remove real duplication or enforce a boundary already present in the architecture docs.
- New backend work must follow the backend MVVM-style layering rule in [Implementation Contracts](docs/architecture/implementation-contracts.md#114-backend-mvvm-style-layering-rule); do not add new business logic to oversized service root files.

## Review Expectations

Every PR should explain:

- what changed
- why it changed
- how it was validated
- what operational risk remains
- whether generated contracts, migrations, or docs changed

Security-sensitive changes should also explain secret handling, tenant isolation, audit impact, and failure behavior.
