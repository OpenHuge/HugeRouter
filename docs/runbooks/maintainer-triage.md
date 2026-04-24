# Maintainer Triage

[Back to Docs Index](../README.md)

This runbook turns common practices from mature open source infrastructure projects into a lightweight HugeRouter triage workflow.

## Intake Principles

- Keep issues reproducible and scoped.
- Route work by ownership boundary before implementation starts.
- Treat security, tenant isolation, billing correctness, and data-plane reliability as escalation paths.
- Prefer small PRs that preserve reviewability over broad refactor batches.

## Labels

Use these label groups consistently:

- `area:gateway`
- `area:control-plane`
- `area:console`
- `area:provider-adapter`
- `area:protocol-contract`
- `area:ledger-billing`
- `area:runtime-ci`
- `area:docs`
- `kind:bug`
- `kind:feature`
- `kind:task`
- `priority:p0`
- `priority:p1`
- `priority:p2`
- `status:needs-repro`
- `status:blocked`
- `status:ready`

GitHub issue templates start with broad labels such as `bug`, `enhancement`, and `task`; maintainers can add the normalized labels during triage.

## P0 Escalation

Treat an issue as P0 when it affects:

- tenant data isolation
- provider secret exposure
- gateway request-path availability
- billing or ledger idempotency
- audit record loss
- route receipt or replay capsule redaction
- release pipeline integrity

P0 work should get a focused branch and minimal unrelated churn.

## Review Routing

- Gateway/protocol changes should include backend and protocol reviewers.
- Console changes should include frontend reviewers and, when auth is involved, security reviewers.
- Runtime/CI changes should include platform reviewers.
- Contract changes should include both producer and consumer owners.

## Done Criteria

A task is done when:

- the relevant local checks pass
- generated artifacts are updated or explicitly unchanged
- docs or runbooks reflect changed operator behavior
- the PR body lists remaining risk
- reviewers can understand the change without reconstructing hidden context
