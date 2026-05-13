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
- `provider_resources` must be able to represent both official upstream APIs and third-party relay or gateway systems that front those APIs

Recommended endpoint shapes:

```text
GET    /v1/provider-resources
POST   /v1/provider-resources
GET    /v1/provider-resources/:providerResourceId
PATCH  /v1/provider-resources/:providerResourceId
GET    /v1/codex-auth-accounts
POST   /v1/codex-auth-accounts
GET    /v1/oauth-sharing-leases
POST   /v1/oauth-sharing-leases
POST   /v1/oauth-sharing-leases/:leaseId/revoke
GET    /v1/oauth-carpools
POST   /v1/oauth-carpools
DELETE /v1/oauth-carpools/:carpoolId
GET    /v1/oauth-sharing-usage
POST   /internal/gateway/oauth-pool/select
POST   /internal/gateway/oauth-pool/runtime-leases/:runtimeLeaseId/heartbeat
POST   /internal/gateway/oauth-pool/runtime-leases/:runtimeLeaseId/release
POST   /internal/gateway/oauth-pool/accounts/:accountId/feedback
POST   /internal/gateway/codex-account-pool/lease

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

Bedrock provider resources use the same payload shape with `provider_id="bedrock"` and a regional Bedrock Runtime endpoint:

```json
{
  "provider_resource_id": "prvrsrc_bedrock_claude",
  "provider_id": "bedrock",
  "name": "Bedrock Claude",
  "status": "active",
  "provenance_class": "official_api",
  "credential_owner_type": "platform",
  "deployment_scope": "shared",
  "region": "us-east-1",
  "endpoint_base_url": "https://bedrock-runtime.us-east-1.amazonaws.com",
  "auth_kind": "api_key",
  "health_state": "healthy",
  "capabilities": {
    "supports_streaming": false,
    "supports_tool_calling": false,
    "supports_json_mode": false,
    "supports_realtime": false,
    "supports_response_model_metadata": true
  },
  "supported_protocol_families": ["openai_chat"],
  "is_transit_gateway": false
}
```

The native Bedrock adapter ignores the generic provider API key and uses the AWS SDK credential chain for credentials and SigV4 signing.

Provider-resource guardrails:

- `provider_id` identifies the canonical provider family or gateway family, not just a marketing site name
- `provenance_class` must distinguish `official_api` from relay-oriented classes such as `third_party_gateway`
- auth, passthrough headers, and sticky-session requirements must be modeled explicitly when the upstream is another gateway
- relay compatibility should be captured with a typed flavor field such as `generic_openai_compatible`, `one_api_like`, `new_api_like`, `sub2api_like`, `litellm_like`, or `lmrouter_like`

### 21.3.2.1 Relay and Gateway Provider Resource Example

```json
{
  "provider_resource_id": "prvrsrc_456",
  "provider_id": "gateway_openai_compatible",
  "name": "relay-cn-primary",
  "status": "active",
  "provenance_class": "third_party_gateway",
  "credential_owner_type": "tenant",
  "deployment_scope": "dedicated",
  "region": "cn-east",
  "endpoint_base_url": "https://relay.example.com/v1",
  "auth_kind": "api_key",
  "health_state": "healthy",
  "quota_policy_id": "budgetpol_456",
  "metadata": {
    "gateway_flavor": "sub2api_like",
    "canonical_upstream_family": "openai",
    "model_discovery_mode": "openai_models_list",
    "header_strategy": "bearer_or_x_api_key",
    "preserve_headers": ["session_id"],
    "sticky_session_supported": true,
    "supports_streaming": true
  },
  "created_at": "2026-04-20T00:00:00Z",
  "updated_at": "2026-04-20T00:00:00Z"
}
```

This shape is intended for relay panels and upstream gateway sites that generate and manage their own downstream keys. The control plane should store HugeRouter's secret reference to that upstream relay key, not expose the relay key itself as a customer credential.

Flavor guidance:

- use `generic_openai_compatible` when HugeRouter only relies on `/v1/*` behavior and no family-specific quirks
- use `one_api_like` or `new_api_like` when the upstream panel family is known and HugeRouter needs family-specific discovery or diagnostics behavior
- use `sub2api_like` for account-pool or subscription relay systems
- use `litellm_like` or `lmrouter_like` for broker gateways that already perform their own provider routing behind one key

### 21.3.3 Codex Auth Account Pool

`POST /v1/codex-auth-accounts` accepts an uploaded Codex `auth.json`, creates or reuses a `chatgpt_web` transit provider resource, and stores the credential payload encrypted with AES-256-GCM. Public responses only include account metadata, the SHA-256 fingerprint, and the encryption key id.

```json
{
  "display_name": "Team Codex account",
  "tenant_id": "tenant_acme",
  "project_id": "proj_core",
  "provider_resource_id": "prvrsrc_codex_team",
  "endpoint_base_url": "https://chatgpt-reverse-proxy.example.com/v1",
  "region": "global",
  "auth_json": {
    "tokens": {
      "access_token": "redacted",
      "refresh_token": "redacted"
    }
  }
}
```

Reverse-proxy workers lease an account through `POST /internal/gateway/codex-account-pool/lease` with `Authorization: Bearer $CONTROL_PLANE_INTERNAL_TOKEN`. That internal response includes the decrypted `auth_json` and must stay on a private network path. The lease path now delegates to the runtime-owned OAuth pool selector and accepts optional `lease_id`, `carpool_id`, `borrower_workspace_id`, `session_id`, `model_id`, `holder_id`, `operation_id`, `lease_ttl_seconds`, and `binding_policy` fields. Selection is strict: an expired, revoked, paused, over-budget, over-concurrency, rate-limited, overloaded, temp-unschedulable, disabled, unhealthy, or unauthorized pool context returns a blocked reason and never falls back to an unrelated account. Successful leases return runtime metadata including `runtime_lease_id`, `binding_id`, `binding_expires_at`, and `fencing_token`.

`POST /internal/gateway/oauth-pool/select` exposes the same selector without decrypted credentials. It returns a public account view plus `reason`, `lease_id`, `carpool_id`, `runtime_lease_id`, `binding_id`, `binding_expires_at`, and `fencing_token` diagnostics, and never includes plaintext API keys or OAuth tokens. Workers keep runtime leases fresh with `POST /internal/gateway/oauth-pool/runtime-leases/:runtimeLeaseId/heartbeat`, release capacity with `POST /internal/gateway/oauth-pool/runtime-leases/:runtimeLeaseId/release`, and report success or provider errors with `POST /internal/gateway/oauth-pool/accounts/:accountId/feedback`.

### 21.3.4 Authorized Account Pool Sharing

Sharing is modeled as authorized account pool sharing, team quota carpools, and router scheduling. It is not a public account resale surface and must not be used to hide real usage, bypass provider controls, or avoid rate limits.

`POST /v1/oauth-sharing-leases` upserts a runtime-owned sharing lease:

```json
{
  "lease_id": "lease_codex_acme_support",
  "owner_workspace_id": "tenant_acme",
  "borrower_workspace_id": "tenant_acme",
  "provider": "codex",
  "pool_id": "prvrsrc_codex_team",
  "allowed_account_ids": ["codexacct_123"],
  "status": "active",
  "starts_at": "2026-04-22T00:00:00Z",
  "expires_at": "2026-05-22T00:00:00Z",
  "max_concurrent_runs": 2,
  "usage_budget": {
    "turns": 100
  },
  "policy": "fair_share",
  "metadata": {}
}
```

`POST /v1/oauth-carpools` upserts a team quota carpool:

```json
{
  "carpool_id": "carpool_codex_acme",
  "provider": "codex",
  "name": "Acme Codex Team Share",
  "member_workspace_ids": ["tenant_acme"],
  "pool_ids": ["prvrsrc_codex_team"],
  "strategy": "fair_share",
  "member_weights": {},
  "per_member_concurrency_limit": 2,
  "per_member_turn_budget": 50,
  "enabled": true,
  "metadata": {}
}
```

`GET /v1/oauth-sharing-usage` returns runtime-owned usage aggregates and audit events for selects, revokes, and budget blocks. Audit metadata is intentionally secret-free.

### 21.3.5 Route Simulation Request and Response

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
