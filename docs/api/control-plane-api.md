# 21. Control Plane API

[Back to Docs Index](../README.md)

The control plane API is used by the frontend and potentially automation clients.

### 21.1 Resource Families

- tenants
- projects
- users and service accounts
- API credentials
- providers
- provider resources
- model aliases
- route policies
- policy bundles
- budgets
- pricing tables
- usage reports
- ledger adjustments
- audit records
- webhooks

### 21.2 API Style

- JSON over HTTPS
- OpenAPI-described
- cursor pagination
- optimistic concurrency control for mutable resources
- consistent error envelope
- prefer runtime-effective resources over exhaustive admin surface area in the first implementation

### 21.3 Example Endpoints

```text
GET    /v1/tenants
POST   /v1/tenants
GET    /v1/tenants/:tenantId
PATCH  /v1/tenants/:tenantId

GET    /v1/projects
POST   /v1/projects

GET    /v1/providers
POST   /v1/providers

GET    /v1/route-policies
POST   /v1/route-policies

GET    /v1/usage/summary
GET    /v1/ledger/entries
POST   /v1/ledger/adjustments

GET    /v1/audit/events
GET    /v1/health/routes
```

### 21.3.1 Priority V1 Resource Surfaces

The first control-plane implementation should usually prioritize these resources:

- `provider_resources`
- `route_policies`
- `budget_policies`
- `config_snapshots`
- `route_simulations`
- `route_receipts`
- `replay_capsules`

Guardrail:

- if a control-plane resource cannot yet affect routing, admission, billing, or supportability, it should usually remain out of the first implementation slice

Recommended endpoint shapes:

```text
GET    /v1/provider-resources
POST   /v1/provider-resources
GET    /v1/provider-resources/:providerResourceId
PATCH  /v1/provider-resources/:providerResourceId

GET    /v1/budget-policies
POST   /v1/budget-policies
GET    /v1/budget-policies/:budgetPolicyId
PATCH  /v1/budget-policies/:budgetPolicyId

POST   /v1/route-simulations
GET    /v1/route-receipts/:routeReceiptId
GET    /v1/replay-capsules/:replayCapsuleId
GET    /v1/config-snapshots/:configSnapshotId
POST   /v1/config-snapshots/:configSnapshotId/activate
```

### 21.3.2 Provider Resource Payload Contract

```json
{
  "provider_resource_id": "prvrsrc_123",
  "provider_id": "openai",
  "name": "openai-us-east-primary",
  "status": "active",
  "provenance_class": "official_api",
  "credential_owner_type": "platform",
  "deployment_scope": "shared",
  "region": "us-east-1",
  "endpoint_base_url": "https://api.openai.com/v1",
  "auth_kind": "api_key",
  "health_state": "healthy",
  "quota_policy_id": "budgetpol_123",
  "cost_policy_id": "price_123",
  "metadata": {
    "supports_streaming": true
  },
  "created_at": "2026-04-20T00:00:00Z",
  "updated_at": "2026-04-20T00:00:00Z"
}
```

### 21.3.3 Route Simulation Request and Response

`POST /v1/route-simulations`

Request:

```json
{
  "tenant_id": "tenant_123",
  "project_id": "proj_123",
  "credential_scope": "cred_123",
  "protocol_family": "openai_chat",
  "model_alias": "reasoning-fast",
  "required_capabilities": ["tool_calling", "json_mode"],
  "region": "us-east-1",
  "expected_prompt_tokens": 12000,
  "expected_max_output_tokens": 4000,
  "traffic_class": "interactive"
}
```

Response:

```json
{
  "simulation_id": "routesim_123",
  "config_snapshot_id": "cfgsnap_123",
  "admission_result": "admitted",
  "eligible_candidates": [
    {
      "provider_resource_id": "prvrsrc_123",
      "score_breakdown": {
        "latency": 0.82,
        "cost": 0.66,
        "health": 0.97,
        "trust": 1.0
      }
    }
  ],
  "excluded_candidates": [
    {
      "provider_resource_id": "prvrsrc_456",
      "reason": "rejected_provenance_class"
    }
  ],
  "selected_target": "prvrsrc_123",
  "estimated_cost": {
    "currency": "USD",
    "amount": "0.1420"
  }
}
```

### 21.3.4 Optimistic Concurrency Contract

Mutable resources should expose a monotonic `version` field and require either:

- `If-Match: "<version>"`
- or a request body `version`

Writes with stale versions must fail with `409 conflict`.

Implementation freedom:

- a thinner first slice may use `updated_at` or revision hashes instead of integer versions
- `If-Match` and body-level versioning do not both need to exist in the first release as long as one clear concurrency contract is enforced

### 21.3.5 Error Code Registry for V1

The first implementation should standardize these control-plane error codes:

- `validation_failed`
- `conflict`
- `not_found`
- `forbidden`
- `invalid_provenance_transition`
- `snapshot_not_active`
- `simulation_failed`
- `resource_quarantined`

### 21.4 API Error Envelope

```json
{
  "error": {
    "code": "route_not_available",
    "message": "No eligible route target matched the request.",
    "request_id": "req_123",
    "details": {
      "model_alias": "reasoning-fast"
    }
  }
}
```


---


## 36. API and Schema Generation

### 36.1 OpenAPI

The control plane API must publish OpenAPI specs.

### 36.2 Type Generation

Generated assets should include:

- TypeScript API client
- frontend schema helpers where safe
- example payloads for docs

### 36.3 Schema Governance

Schema changes must be reviewed as first-class API changes.

---
