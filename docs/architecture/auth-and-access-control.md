# 13. Authentication and Authorization

[Back to Docs Index](../README.md)

### 13.1 Northbound Authentication

Supported mechanisms:

- API keys
- JWT bearer tokens
- signed ephemeral session tokens
- mTLS for enterprise/private ingress

### 13.2 Credential Scope

Credentials may be scoped to:

- tenant
- project
- environment
- route set
- model alias set
- budget policy
- IP restriction
- region restriction
- expiration time

### 13.3 Authorization Model

The control plane should implement RBAC with optional ABAC-style condition support.

Suggested roles:
- platform admin
- tenant owner
- tenant admin
- billing admin
- security admin
- developer
- read-only auditor

### 13.4 Key Management

- keys must be hashed at rest where possible
- key prefixes should be public and searchable
- secret material should be reveal-once where appropriate
- upstream secrets must be stored in a secret store or envelope-encrypted
- key rotation must be supported without downtime

### 13.5 Virtual Key Model

Following the strongest patterns from modern AI gateways, the system should distinguish between:

- consumer-facing gateway keys
- upstream provider credentials
- ephemeral session credentials

Gateway-issued virtual keys should be the normal northbound credential model.

A virtual key may carry:

- tenant and project scope
- allowed model aliases
- route-set restrictions
- budget association
- expiration and rotation policy
- audit metadata such as creator and purpose

Provider secrets should never be reused as customer-facing credentials.

### 13.6 Budget and Override Controls

Authentication and budget governance are closely linked in AI gateways.

The control plane should support:

- default budget policy at project or environment level
- per-key overrides
- temporary budget increases with expiry
- emergency revocation without rotating unrelated credentials

Every override should be audited.

### 13.7 Plane-to-Plane Trust

Borrowing from mature gateway CP/DP patterns, communication between control-plane and data-plane services should support:

- mTLS for private deployments
- explicit service identity
- version compatibility checks where configuration snapshots cross service boundaries
- least-privilege credential distribution

The hot path must not require direct access to raw control-plane credentials stores.

---
