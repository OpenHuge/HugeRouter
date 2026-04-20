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
