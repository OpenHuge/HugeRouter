# 13. Authentication and Authorization

[Back to Docs Index](../README.md)

### 13.1 Northbound Authentication

Supported mechanisms:

- API keys
- JWT bearer tokens
- signed ephemeral session tokens
- mTLS for enterprise/private ingress
- agent identity tokens (for non-human agentic callers via A2A or MCP)

Human console authentication should be modeled separately from programmatic northbound credentials.

The admin and tenant console should support these sign-in methods:

- email login
- GitHub OAuth login
- Google OAuth login
- WeChat OAuth login

Recommended policy:

- link all interactive login methods to one internal user identity record
- require verified email before account activation when the provider exposes it
- allow tenant administrators to restrict which providers may be used in their workspace
- support provider binding and unbinding without breaking audit history
- create the same RBAC and tenant membership context regardless of which login provider was used

Recommended default UX:

- email login should prefer passwordless verification such as magic links or one-time codes
- GitHub and Google should be first-class global OAuth providers for developer and enterprise teams
- WeChat should be first-class for China-facing tenants that need a familiar local identity provider
- every successful console login should mint a HugeRouter-managed session rather than treating upstream OAuth tokens as the product session model

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

Delegated and non-human credentials should additionally support:

- caller identity type (`human`, `service`, `agent`)
- delegated subject or originating principal
- maximum delegation depth
- allowed protocol families such as `responses`, `realtime`, `mcp`, or `a2a`
- allowed tool or agent target classes

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
- agent (non-human autonomous identity with constrained scope)

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

Recommended default:

- the gateway should issue and validate its own northbound credentials even when the eventual upstream also uses OAuth or API keys
- remote agent or MCP credentials should be bound to a gateway-recognized principal record before they affect routing or tool-use policy

Guardrail:

- do not let upstream bearer material become the platform's de facto customer identity model

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

### 13.8 Non-Human Identity (NHI) Management

As agents become first-class principals (per OWASP Agentic Top 10 2026), the platform must support:

- explicit agent identity registration with capability declarations
- agent credentials with mandatory expiration, scope, and budget policies
- distinguishable audit trails for agent vs. human actions
- A2A Agent Card validation as part of inbound authentication for inter-agent traffic
- delegation chain tracking to trace multi-hop agent interactions back to the originating human or service principal
- automatic credential rotation and lifecycle management for agent identities

Recommended delegation model:

- separate the authenticated caller from the effective acting principal
- record both the immediate caller and the originating subject when delegation is present
- reject delegation chains that exceed configured depth or attempt privilege expansion
- preserve delegation chain identifiers in route receipts, audit artifacts, and async message context

MCP and A2A boundary rule:

- MCP auth should prove who may use a tool or capability surface
- A2A auth should prove who may delegate, accept, or continue a task
- do not assume one credential can safely imply both permissions without an explicit binding policy

### 13.9 Human Identity Lifecycle

Interactive human identities should support:

- first-login account creation with tenant invitation checks
- account linking across email, GitHub, Google, and WeChat when the verified email or admin policy allows it
- explicit tenant membership review before granting console access
- session revocation on role change, provider unlink, or security events
- step-up verification for security-sensitive actions such as credential rotation or billing changes

Minimum audit fields for human authentication events:

- internal user id
- tenant id
- provider (`email`, `github`, `google`, `wechat`)
- provider subject id
- login result and failure reason
- session id
- source IP, user agent, and timestamp

---
