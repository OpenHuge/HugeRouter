# 22. Public Gateway API

[Back to Docs Index](../README.md)

The public gateway API is the request ingress for customer workloads.

### 22.1 Public API Principles

- protocol compatibility where declared
- avoid inventing a gateway-only request dialect when the goal is compatibility with an existing upstream protocol family
- explicit `ws://` and `wss://` ingress for bidirectional streaming and Realtime APIs
- request tracing headers
- streaming support (both HTTP chunked and persistent WebSockets)
- explicit error normalization
- stable auth and rate limit semantics

### 22.2 Public API Features

- logical model aliasing
- protocol-native request acceptance
- tenant/project isolation
- streaming and non-streaming modes
- **Bidirectional WebSocket Proxying:** Maintain stateful connections, apply in-flight frame inspection, and proxy tool execution contexts (e.g., via MCP) without breaking the stream.
- usage and request IDs in response metadata where compatible
- progressive metering events emitted periodically during long-lived `ws` sessions
- optional debug headers in approved environments

### 22.3 Supported Ingress Endpoints

1. **REST / HTTP Streaming:** `https://api.gateway.local/v1/chat/completions` (OpenAI compatible)
2. **Responses API:** `POST /v1/responses` (OpenAI Responses-compatible text ingress)
3. **Images API:** `POST /v1/images/generations` (OpenAI Images-compatible generation ingress using `openai_images` route policies)
4. **WebSocket / Realtime:** `wss://api.gateway.local/v1/realtime` (for voice/audio and continuous agent interactions)
5. **Ephemeral Token Generation:** `POST /v1/realtime/client_secrets` (to allow secure browser WebRTC/WS connections without exposing master keys)

### 22.3.1 HugeCode Local Commercial Routing Support

The gateway also exposes the local HugeCode integration contract:

- `GET /health`
  Lightweight liveness probe. `/healthz` remains supported for compatibility.
- `GET /ready`
  Readiness probe returning `service`, `status`, `baseUrl`, `routeBaseUrl`, `capabilities`, and `diagnostics`.
- `GET /v1/hugerouter/commercial-service`
  Local HugeRouter commercial service snapshot aligned to HugeCode's `HugeRouterCommercialServiceSnapshot` shape. This endpoint may return dev-backed local account state, but it must not claim to be a production billing or subscription source of truth.
- `POST /v1/hugerouter/route-tokens`
  Issues a route token aligned to HugeCode's `HugeRouterRouteTokenIssueRequest` and `HugeRouterRouteTokenIssueResponse` shapes.

Route token invariants:

- issued tokens use the `hgrt_` prefix and are returned only in the issue response
- commercial service snapshots return only token summary metadata and never the plaintext token
- gateway storage must retain only a hash or equivalent irreversible token digest
- `Authorization: Bearer hgrt_...` is accepted by `POST /v1/responses`, `POST /v1/chat/completions`, and `POST /v1/images/generations`
- invalid, expired, or revoked route tokens return `auth_invalid`
- route tokens without required routing/provider scopes return `auth_insufficient_scope`

### 22.4 Preferred Response Metadata

Where protocol compatibility permits, the gateway should emit:

- `x-request-id`
- `x-trace-id`
- `x-route-receipt-id`
- `x-config-snapshot-id`

Invariant:

- at least `x-request-id` should be present on stable HTTP surfaces

Recommended default:

- include the other headers when they are already available without distorting protocol compatibility

### 22.5 Debug Modes

A controlled debug mode may expose:

- selected route target
- retry count
- route policy ID
- upstream provider class

This must be disabled by default for public production traffic.

Recommended debug headers in approved environments:

- `x-debug-selected-target`
- `x-debug-route-policy-id`
- `x-debug-fallback-count`
- `x-debug-admission-result`

### 22.6 Canonical Public Error Codes for V1

The first implementation should normalize upstream and policy failures into a stable gateway-facing set:

- `auth_invalid`
- `auth_insufficient_scope`
- `route_not_available`
- `budget_exceeded`
- `rate_limited`
- `concurrency_limited`
- `provider_unavailable`
- `upstream_timeout`
- `upstream_protocol_error`
- `request_validation_failed`

### 22.7 Realtime Client Secret Contract

`POST /v1/realtime/client_secrets` should return:

```json
{
  "client_secret": "rtm_secret_123",
  "expires_at": "2026-04-20T00:15:00Z",
  "request_id": "req_123",
  "allowed_session_params": {
    "model_aliases": ["realtime-default"],
    "max_duration_seconds": 900,
    "max_concurrency": 1
  }
}
```

The secret should be:

- scoped to a tenant, project, and principal
- time-limited
- non-refreshable
- revocable

### 22.7.1 Current Realtime Gateway MVP Scope

The first standalone `services/realtime-gateway` slice currently supports:

- `GET /v1/realtime` over WebSockets only
- bearer-token admission using signed ephemeral realtime tokens
- explicit `realtime.connect` scope enforcement
- explicit expiry and parameter validation for `model` and session duration
- lifecycle tracing for `connect`, `accepted`, `rejected`, `closed`, and `upstream-failed`

Deliberate boundary:

- handshake admission does **not** perform billing or usage metering mutations
- metering should begin only after the session is accepted and later execution/usage events are emitted

### 22.8 First Implementation Acceptance Semantics

Agents implementing the first public gateway should preserve these invariants:

- every accepted request yields a `request_id`
- every routed request yields a `route_receipt_id`
- every policy or budget rejection returns a normalized error code instead of raw upstream text
- debug headers must never leak upstream secrets or raw prompt fragments
- retries must not silently alter tenant-visible auth or billing identity

Implementation freedom:

- a thin first release may surface some metadata in response bodies or logs instead of headers for protocol families where header compatibility is awkward
- realtime support can begin with a smaller parameter surface if token scoping and expiry semantics are preserved

---
