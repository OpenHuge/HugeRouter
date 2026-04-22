# Track 00 - Foundation And Quality

## Mission

Make the repository reproducible, testable, and enforceable so that every other track can ship against a stable baseline instead of fighting local environment drift and missing CI.

## Current Baseline

- The repo pins Rust `1.95.0` in `rust-toolchain.toml`, but the current local machine is on `rustc 1.94.1`, which blocks all backend verification before compilation starts.
- `package.json`, `turbo.json`, and `justfile` provide a working monorepo baseline, but `.github/workflows` is not present yet.
- `.devcontainer` and `infra/docker/compose.yaml` already define a credible local stack for Node, Rust, PostgreSQL, Redis, NATS, OpenTelemetry, Prometheus, and Grafana.
- Several workspace test scripts still succeed with placeholder output instead of real checks.

## Owned Paths

- `package.json`
- `pnpm-workspace.yaml`
- `turbo.json`
- `justfile`
- `rust-toolchain.toml`
- `.devcontainer/*`
- `.github/workflows/*`
- `infra/*`
- `scripts/*` when the change is about root verification or developer workflow
- documentation updates directly tied to these paths

## Do Not Edit

- `crates/core-domain`
- `crates/protocol-ir`
- `services/control-plane-api`
- `services/gateway-api`
- `apps/console-web`
- `packages/ui-kit`
- `packages/ts-api-client`
- `packages/ts-shared-schema`

If another track needs a root-script hook added, expose a stable command surface rather than editing their implementation area.

## Deliverables

1. Reproducible Rust and Node toolchain story across local shell, devcontainer, and CI.
2. Real CI workflows for lint, typecheck, test, and build on `main` and PRs.
3. Clear quality gates for placeholder/no-op tests so they do not look like meaningful coverage.
4. Faster, better-scoped root commands for local development and CI.
5. Updated runbook notes where commands or environment expectations change.

## Ordered Plan

1. Align the toolchain entry points.
   - Verify `rust-toolchain.toml`, `.devcontainer`, and root scripts agree on Rust and Node versions.
   - Decide whether local bootstrap should hard-fail with a clearer message when the wrong Rust toolchain is installed.
   - Add one obvious verification command for contributors to confirm they are on the supported toolchain.
2. Create CI workflows.
   - Add separate jobs for JavaScript lint/typecheck/test/build and Rust fmt/clippy/test/check.
   - Use caching for `pnpm`, Cargo registry, Cargo git, and Turbo outputs where worthwhile.
   - Keep jobs readable and split by failure mode instead of one monolithic workflow step.
3. Tighten root task behavior.
   - Make root commands report meaningful failures.
   - Revisit the order of `package.json` and `justfile` commands so failures surface predictably.
   - Ensure Turbo tasks define sensible outputs and do not emit noisy warnings by default.
4. Address placeholder verification gaps.
   - Decide which packages should be allowed to have explicit smoke tests for now versus which must gain real tests before their track is complete.
   - Prevent green CI from implying coverage exists where it does not.
5. Document the stable workflow.
   - Update the relevant runbooks with the exact bootstrap, lint, test, and local stack flow.
   - Note the minimum expectations for future tracks to add new packages or services.

## Required Tests

- Workflow validation using local dry runs where practical.
- Root command verification:
  - `pnpm lint`
  - `pnpm typecheck`
  - `pnpm test`
  - `pnpm build`
  - `cargo check --workspace`
  - `cargo test --workspace`
- If local Rust `1.95.0` is still unavailable, verify the Rust commands inside the devcontainer or CI-equivalent environment and document the exact method.

## Definition Of Done

- A new contributor can follow one documented path and get the workspace into a supported state.
- CI exists and exercises the same command surface the team uses locally.
- The repo no longer reports misleading success for placeholder checks without making that status explicit.
- Turbo, Cargo, and devcontainer settings do not contradict each other on versions or expected commands.
- All documentation changes are scoped to the commands and workflows actually shipped.

## Branch And PR Convention

- Branch: `codex/track-00-foundation-quality`
- PR title: `[Track 00] Stabilize toolchain and quality gates`
- Required PR notes:
  - exact environments used for verification
  - whether Rust was validated locally, in devcontainer, or in CI
  - any remaining temporary exceptions for placeholder packages
