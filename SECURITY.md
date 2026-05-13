# Security Policy

## Reporting Vulnerabilities

Do not open public GitHub issues for suspected vulnerabilities.

Report security concerns privately through the repository maintainers or the owning organization security contact. Include:

- affected component or endpoint
- reproduction steps
- expected impact
- whether secrets, tenant data, credentials, route receipts, usage events, or audit records may be exposed
- any logs or traces that are safe to share

If you are unsure whether something is security-sensitive, report it privately first.

## Security-Sensitive Areas

The following areas require extra review:

- API key issuance, hashing, rotation, and scope resolution
- control-plane to gateway trust boundaries
- provider credential handling
- tenant/project authorization
- route receipt, replay capsule, audit, and usage event redaction
- billing, budget, and ledger idempotency
- NATS event subjects and worker consumption semantics
- console redirect, session, and OAuth callback handling

## Development Expectations

- Never commit real provider keys, tenant secrets, OAuth secrets, session cookies, or production data.
- Prefer generated test credentials and local-only fixtures.
- Keep sensitive examples masked.
- Add tests for tenant isolation, invalid credentials, replay/idempotency, and degraded dependency behavior when a change touches those paths.

## Supported Versions

This repository is pre-1.0. Security fixes target the default branch unless a release branch is explicitly documented.
