# 8. Domain Model

[Back to Docs Index](../README.md)

The platform's core domain objects are listed below.

### 8.1 Tenant

A top-level organizational boundary.

Attributes:
- `tenant_id`
- `name`
- `status`
- `billing_plan`
- `default_region_policy`
- `default_security_policy`
- `created_at`
- `updated_at`

### 8.2 Project

A scoped sub-boundary inside a tenant, often mapped to an application or environment.

Attributes:
- `project_id`
- `tenant_id`
- `name`
- `slug`
- `environment`
- `default_route_policy_id`
- `default_budget_policy_id`

### 8.3 Principal

A user, service account, API key subject, or machine identity.

### 8.4 API Credential

Represents a northbound access credential.

Types:
- user API key
- project API key
- ephemeral session token
- JWT-based machine token
- mTLS client identity

### 8.5 Provider

Represents an upstream provider family.

Examples:
- OpenAI
- Anthropic
- Google Gemini
- xAI
- Bedrock
- Vertex AI
- upstream gateway / transit system

### 8.6 Provider Resource

A specific upstream access resource.

Examples:
- API key
- OAuth token
- account pool entry
- subscription-backed account
- region-specific gateway endpoint

### 8.7 Route Policy

Defines how requests should be matched and routed.

### 8.8 Model Alias

A stable logical model name exposed to users.

Examples:
- `gpt-4.1` as a logical alias
- `reasoning-fast`
- `vision-default`
- `auto-chat-premium`

### 8.9 Capability Profile

A structured representation of what a route target supports.

### 8.10 Usage Event

Immutable record describing measured consumption for a request or session.

### 8.11 Ledger Entry

Immutable accounting event that affects projected balances or cost reporting.

### 8.12 Audit Event

Immutable security or governance event.

---
