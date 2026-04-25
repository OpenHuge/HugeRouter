# Route Gateway Standalone Branch

[Back to Docs Index](../README.md)

## Status

The route gateway / AI relay product line is no longer part of the Phase 1 main branch.

The main branch is now scoped to the trusted AI resource trading platform. Gateway runtime services, realtime gateway runtime, public gateway examples, and the upstream transit-gateway adapter were removed from main to keep the first product line smaller and easier to ship.

## Removed From Main

- `services/gateway-api`
- `services/realtime-gateway`
- `crates/provider-gateway`
- public gateway API documentation
- gateway protocol example payloads
- gateway load/failure scripts
- runtime Docker and CI references that built `gateway-api`

## Remaining Boundary

Some control-plane concepts still exist because the marketplace needs redacted quality evidence:

- trial connection records
- relay evaluation records
- replay capsules
- route receipt identifiers attached to quality evidence
- summary diagnostics suitable for support handoff

These are marketplace evidence contracts, not a public gateway product surface.

## Interface Rule

If the standalone gateway branch is developed again, it should integrate with the main site through explicit APIs:

- quality probe request
- quality probe result
- replay capsule lookup
- diagnostics summary lookup
- audit evidence attachment

It must not directly own seller verification, listings, escrow state, settlement, or disputes.
