# Control Plane and Console

[Back to Execution Workstreams](README.md)

Deliver the management API and the TanStack Start + HeroUI console for tenants, providers, routes, and billing surfaces.

## Task sequence

| Task ID | Phase | Title                                                                        | Depends On                         |
| ------- | ----- | ---------------------------------------------------------------------------- | ---------------------------------- |
| CTL-001 | PI-0  | Create control plane API service skeleton with typed CRUD conventions        | FND-001, FND-004                   |
| CTL-004 | PI-0  | Build TanStack Start + HeroUI application shell and authenticated app layout | FND-002, FND-006                   |
| CTL-002 | PI-1  | Implement tenant, project, environment, and API key CRUD                     | CTL-001, DB-001, SEC-001           |
| CTL-003 | PI-1  | Implement provider, credential, route policy, and route set management APIs  | CTL-001, PAD-001, RTE-001, SEC-003 |
| CTL-005 | PI-1  | Implement tenant/project management UI flows                                 | CTL-002, CTL-004                   |
| CTL-006 | PI-2  | Implement provider and route management UI with diagnostics surfaces         | CTL-003, CTL-004, RTE-005          |
| CTL-007 | PI-3  | Build usage, budget, and billing dashboards                                  | MET-004, MET-005, CTL-004          |

## Detailed tasks

### CTL-001 - Create control plane API service skeleton with typed CRUD conventions

**Phase:** PI-0  
**Estimated size:** M  
**Recommended owners:** control-plane

**Depends on:** FND-001, FND-004

**Primary paths to touch:**

- `services/control-plane-api`
- `crates/sdk-server`
- `schemas/openapi`

**Expected outputs:**

- service bootstrap
- error envelope
- pagination/filtering conventions

**Acceptance criteria:**

- service exposes versioned base path
- OpenAPI spec is generated from implementation
- integration test can start service against local stack

**Implementation notes:**

- Generate the TS client from OpenAPI and prefer typed server functions or route loaders.
- Do not hardcode enum values that are owned by the backend schema.
- Guard destructive operations with explicit confirmation UX.

### CTL-004 - Build TanStack Start + HeroUI application shell and authenticated app layout

**Phase:** PI-0  
**Estimated size:** M  
**Recommended owners:** frontend-console

**Depends on:** FND-002, FND-006

**Primary paths to touch:**

- `apps/console-web`
- `packages/ui-kit`
- `packages/design-tokens`

**Expected outputs:**

- route tree
- shell layout
- HeroUI-backed theme and navigation system
- shared provider stack and shell primitives

**Acceptance criteria:**

- authenticated and unauthenticated layouts render
- HeroUI-compatible internal shell wrapper is in place
- route-level data loaders compile
- component story coverage exists for shell primitives

**Implementation notes:**

- Generate the TS client from OpenAPI and prefer typed server functions or route loaders.
- Do not hardcode enum values that are owned by the backend schema.
- Guard destructive operations with explicit confirmation UX.
- Build shared layout and feedback primitives in `packages/ui-kit` first, then compose feature pages from those pieces.
- Keep theme decisions centralized in `packages/design-tokens` so route modules do not reconfigure HeroUI locally.

### CTL-002 - Implement tenant, project, environment, and API key CRUD

**Phase:** PI-1  
**Estimated size:** L  
**Recommended owners:** control-plane

**Depends on:** CTL-001, DB-001, SEC-001

**Primary paths to touch:**

- `services/control-plane-api`
- `crates/storage`
- `crates/core-domain`

**Expected outputs:**

- resource CRUD endpoints
- API key issuance/rotation
- resource validation

**Acceptance criteria:**

- CRUD endpoints are idempotent where expected
- API key rotation invalidates old secrets
- OpenAPI and TS client are updated

**Implementation notes:**

- Generate the TS client from OpenAPI and prefer typed server functions or route loaders.
- Do not hardcode enum values that are owned by the backend schema.
- Guard destructive operations with explicit confirmation UX.

### CTL-003 - Implement provider, credential, route policy, and route set management APIs

**Phase:** PI-1  
**Estimated size:** L  
**Recommended owners:** control-plane

**Depends on:** CTL-001, PAD-001, RTE-001, SEC-003

**Primary paths to touch:**

- `services/control-plane-api`
- `crates/storage`
- `crates/provider-traits`
- `crates/routing-engine`

**Expected outputs:**

- provider CRUD
- credential binding
- route policy CRUD

**Acceptance criteria:**

- route policies can be created and validated from API
- secret references are stored without plaintext leakage
- policy schema validation errors are actionable

**Implementation notes:**

- Generate the TS client from OpenAPI and prefer typed server functions or route loaders.
- Do not hardcode enum values that are owned by the backend schema.
- Guard destructive operations with explicit confirmation UX.

### CTL-005 - Implement tenant/project management UI flows

**Phase:** PI-1  
**Estimated size:** M  
**Recommended owners:** frontend-console

**Depends on:** CTL-002, CTL-004

**Primary paths to touch:**

- `apps/console-web/src/routes`
- `packages/ts-api-client`

**Expected outputs:**

- list/detail/create flows
- API key issuance views
- error/loading states

**Acceptance criteria:**

- CRUD flows are fully typed from generated client
- forms validate client-side and server-side
- role-based page guards work

**Implementation notes:**

- Generate the TS client from OpenAPI and prefer typed server functions or route loaders.
- Do not hardcode enum values that are owned by the backend schema.
- Guard destructive operations with explicit confirmation UX.

### CTL-006 - Implement provider and route management UI with diagnostics surfaces

**Phase:** PI-2  
**Estimated size:** L  
**Recommended owners:** frontend-console

**Depends on:** CTL-003, CTL-004, RTE-005

**Primary paths to touch:**

- `apps/console-web/src/routes/providers`
- `apps/console-web/src/routes/routes`

**Expected outputs:**

- provider detail pages
- route editors
- route diagnostics panels

**Acceptance criteria:**

- operators can inspect route health and policy resolution
- form edits are optimistic only where safe
- dangerous operations require confirmation

**Implementation notes:**

- Generate the TS client from OpenAPI and prefer typed server functions or route loaders.
- Do not hardcode enum values that are owned by the backend schema.
- Guard destructive operations with explicit confirmation UX.

### CTL-007 - Build usage, budget, and billing dashboards

**Phase:** PI-3  
**Estimated size:** L  
**Recommended owners:** frontend-console

**Depends on:** MET-004, MET-005, CTL-004

**Primary paths to touch:**

- `apps/console-web/src/routes/usage`
- `apps/console-web/src/routes/billing`

**Expected outputs:**

- usage charts
- budget threshold views
- balance and invoice projections

**Acceptance criteria:**

- dashboards match projection APIs
- time-range filters and tenant scopes work
- large tables are paginated and exportable

**Implementation notes:**

- Generate the TS client from OpenAPI and prefer typed server functions or route loaders.
- Do not hardcode enum values that are owned by the backend schema.
- Guard destructive operations with explicit confirmation UX.
