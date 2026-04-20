# ADR 0006: Why TanStack Start RC with Exact Version Pinning

[Back to ADR Index](README.md)

## Status

Accepted

## Context

As of **April 20, 2026**, TanStack Start is still published as a release candidate rather than a declared stable 1.0 release. At the same time, it aligns well with the project goals:

- type-safe route-first application structure
- server functions and request middleware
- strong TanStack Query and Router integration
- deploy-anywhere posture with Nitro-backed hosting guidance

We need a way to use it now without turning framework churn into constant rework.

## Decision

Use TanStack Start for the console, but pin it to an **exact RC version** during bootstrap and early implementation.

Associated policy:

- no caret ranges for Start packages
- upgrades only through explicit review
- breaking or surprising RC changes are documented in ADRs or upgrade notes
- architecture must keep business logic and UI-kit code portable enough to survive a later framework change if required

## Alternatives Considered

### Wait for 1.0 stable before starting

This would reduce framework risk but would also delay implementation unnecessarily, especially because the surrounding stack decisions are already mature enough to proceed.

### Switch to a more mature framework immediately

That would reduce RC risk, but it would also move the project away from the typed TanStack-centric workflow we have already optimized around.

### Use TanStack Start with loose semver ranges

This creates unnecessary instability in a phase where the team needs reproducible bootstrap behavior more than automatic access to every RC update.

## Consequences

Positive:

- we can begin implementation now
- framework churn is bounded
- local setups and CI remain reproducible
- the application architecture stays aligned with the TanStack ecosystem

Tradeoffs:

- upgrades become explicit work
- some Start internals may still change before 1.0 stable
- we must keep an eye on release notes and migration guidance

## Follow-up Actions

- pin exact Start versions in `package.json`
- record the chosen version in the bootstrap runbook
- keep `src/start.ts`, route modules, and shared UI packages cleanly separated
- re-evaluate after Start 1.0 stable ships
