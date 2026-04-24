# Track 00 - Foundation And Quality

## Mission

Make the repository reproducible, testable, and enforceable so that every other track can ship against a stable baseline instead of fighting local environment drift and missing CI.

## Current Baseline

- The repo now pins Rust `1.94.1` in `rust-toolchain.toml` so local shell, Cargo manifests, and repository documentation agree on the supported baseline.
- `package.json`, `turbo.json`, and `justfile` provide a working monorepo baseline. GitHub Actions workflows now exist for quality and release automation.
- `.devcontainer` and `infra/docker/compose.yaml` already define a credible local stack for Node, Rust, PostgreSQL, Redis, NATS, OpenTelemetry, Prometheus, and Grafana.
- Root quality gates now include file-size budgets, Prettier drift baselining, workspace test-policy checks, JavaScript lint/typecheck/test/build, and Rust fmt/clippy/check/test commands.
- Some package scripts intentionally remain smoke-level or non-generated placeholders. Track `00` should keep making those exceptions explicit so green CI is not mistaken for full behavioral coverage.

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
2. Maintainable CI workflows for lint, typecheck, test, build, release notes, and contract drift checks.
3. Clear quality gates for intentionally shallow package tests so they do not look like meaningful coverage.
4. Faster, better-scoped root commands for local development and CI.
5. Updated runbook notes where commands, tool versions, stack modes, or environment expectations change.

## Ordered Plan

1. Align the toolchain entry points.
   - Verify `rust-toolchain.toml`, `.devcontainer`, and root scripts agree on Rust and Node versions.
   - Decide whether local bootstrap should hard-fail with a clearer message when the wrong Rust toolchain is installed.
   - Add one obvious verification command for contributors to confirm they are on the supported toolchain.
2. Maintain CI workflows.
   - Keep separate jobs for JavaScript lint/typecheck/test/build and Rust fmt/clippy/test/check.
   - Use caching for `pnpm`, Cargo registry, Cargo git, and Turbo outputs where worthwhile.
   - Keep jobs readable and split by failure mode instead of one monolithic workflow step.
3. Tighten root task behavior.
   - Make root commands report meaningful failures.
   - Revisit the order of `package.json` and `justfile` commands so failures surface predictably.
   - Ensure Turbo tasks define sensible outputs and do not emit noisy warnings by default.
4. Address remaining verification gaps.
   - Decide which packages are allowed to keep explicit smoke-level checks for now versus which must gain real tests before their track is complete.
   - Keep `scripts/workspace-test-policy.json` current as packages graduate from placeholders to assertions.
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
- If local Rust `1.94.1` is still unavailable, verify the Rust commands inside the devcontainer or CI-equivalent environment and document the exact method.

## Definition Of Done

- A new contributor can follow one documented path and get the workspace into a supported state.
- CI exists and continues to exercise the same command surface the team uses locally.
- The repo does not report misleading success for placeholder checks without making that status explicit.
- Turbo, Cargo, and devcontainer settings do not contradict each other on versions or expected commands.
- All documentation changes are scoped to the commands and workflows actually shipped.

## Branch And PR Convention

- Branch: `codex/track-00-foundation-quality`
- PR title: `[Track 00] Stabilize toolchain and quality gates`
- Required PR notes:
  - exact environments used for verification
  - whether Rust was validated locally, in devcontainer, or in CI
  - any remaining temporary exceptions for placeholder packages
