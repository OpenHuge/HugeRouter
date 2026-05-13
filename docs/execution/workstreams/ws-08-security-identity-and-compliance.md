# Security, Identity, and Compliance

[Back to Execution Workstreams](README.md)

Define authorization, secret boundaries, SSO, audit trails, and compliance controls.

## Task sequence

| Task ID | Phase | Title                                                                     | Depends On               |
| ------- | ----- | ------------------------------------------------------------------------- | ------------------------ |
| SEC-001 | PI-0  | Define RBAC model, scopes, and authorization middleware                   | FND-001, CTL-001         |
| SEC-003 | PI-1  | Implement secret reference model and provider credential storage boundary | FND-004, DB-001          |
| SEC-004 | PI-2  | Build audit event pipeline and retention controls                         | DB-002, SEC-001, OBS-001 |
| SEC-002 | PI-3  | Implement OIDC/SSO login flow for console users                           | CTL-004, SEC-001         |

## Detailed tasks

### SEC-001 — Define RBAC model, scopes, and authorization middleware

**Phase:** PI-0  
**Estimated size:** M  
**Recommended owners:** security

**Depends on:** FND-001, CTL-001

**Primary paths to touch:**

- `crates/authn-authz`
- `services/control-plane-api`
- `services/gateway-api`

**Expected outputs:**

- role model
- scope evaluation
- authorization middleware

**Acceptance criteria:**

- tenant and platform scopes are distinct
- forbidden responses are normalized
- authorization decisions are traceable

**Implementation notes:**

- Never return plaintext provider secrets from APIs.
- Add audit events for security-sensitive actions.
- Design tenant and platform scopes separately from authentication methods.

### SEC-003 — Implement secret reference model and provider credential storage boundary

**Phase:** PI-1  
**Estimated size:** M  
**Recommended owners:** security, control-plane

**Depends on:** FND-004, DB-001

**Primary paths to touch:**

- `crates/storage`
- `services/control-plane-api`
- `crates/config`

**Expected outputs:**

- secret reference abstraction
- credential vault boundary
- rotation hooks

**Acceptance criteria:**

- plaintext provider secrets are never returned by API
- secret references can be resolved by authorized services only
- rotation is auditable

**Implementation notes:**

- Never return plaintext provider secrets from APIs.
- Add audit events for security-sensitive actions.
- Design tenant and platform scopes separately from authentication methods.

### SEC-004 — Build audit event pipeline and retention controls

**Phase:** PI-2  
**Estimated size:** M  
**Recommended owners:** security, data-platform

**Depends on:** DB-002, SEC-001, OBS-001

**Primary paths to touch:**

- `services/audit-worker`
- `crates/queue`
- `crates/storage`

**Expected outputs:**

- audit event model
- durable audit worker
- retention/archive rules

**Acceptance criteria:**

- security-sensitive actions emit audit events
- audit queries are filterable by actor and resource
- retention policies are testable

**Implementation notes:**

- Never return plaintext provider secrets from APIs.
- Add audit events for security-sensitive actions.
- Design tenant and platform scopes separately from authentication methods.

### SEC-002 — Implement OIDC/SSO login flow for console users

**Phase:** PI-3  
**Estimated size:** M  
**Recommended owners:** security, frontend-console

**Depends on:** CTL-004, SEC-001

**Primary paths to touch:**

- `apps/console-web`
- `services/control-plane-api`
- `crates/authn-authz`

**Expected outputs:**

- OIDC login flow
- session handling
- group-to-role mapping

**Acceptance criteria:**

- interactive login works against a reference IdP
- logout and session expiry are handled cleanly
- role mapping is configurable

**Implementation notes:**

- Never return plaintext provider secrets from APIs.
- Add audit events for security-sensitive actions.
- Design tenant and platform scopes separately from authentication methods.
