# Gateway and Protocol Ingress

[Back to Execution Workstreams](README.md)

Implement the northbound API gateway, protocol IR, protocol-specific ingress layers, and streaming/realtime request handling.

## Task sequence

| Task ID | Phase | Title | Depends On |
|---|---|---|---|
| GWT-001 | PI-1 | Implement gateway HTTP server skeleton and middleware chain | FND-001, FND-003, FND-004 |
| GWT-002 | PI-1 | Implement API key authentication and tenant/project resolution | CTL-001, SEC-001 |
| GWT-003 | PI-1 | Define protocol IR v1 with request, response, and capability models | FND-001 |
| GWT-004 | PI-1 | Ship OpenAI-compatible northbound chat/completions ingress | GWT-001, GWT-003, PAD-001, PAD-002, RTE-002, MET-001 |
| GWT-005 | PI-1 | Implement SSE streaming relay and backpressure-safe response path | GWT-004 |
| GWT-006 | PI-2 | Add Anthropic-native northbound protocol support | GWT-003, PAD-003, RTE-002 |
| GWT-007 | PI-2 | Add Gemini-native northbound protocol support | GWT-003, PAD-004, RTE-002 |
| GWT-008 | PI-4 | Build realtime session gateway skeleton | FND-001, OBS-001, RTE-004 |

## Detailed tasks

### GWT-001 — Implement gateway HTTP server skeleton and middleware chain

**Phase:** PI-1  
**Estimated size:** M  
**Recommended owners:** gateway

**Depends on:** FND-001, FND-003, FND-004

**Primary paths to touch:**
- `services/gateway-api`
- `crates/sdk-server`
- `crates/telemetry`

**Expected outputs:**
- HTTP server bootstrap
- middleware pipeline
- health/readiness endpoints

**Acceptance criteria:**
- service starts with trace and metrics middleware
- health endpoints are covered by tests
- configuration is hot-reload safe or explicitly immutable

**Implementation notes:**
- Keep protocol parsing separate from provider adapter behavior.
- Prefer explicit compatibility errors over silent field dropping.
- Preserve provider-specific metadata in extension fields when possible.

### GWT-002 — Implement API key authentication and tenant/project resolution

**Phase:** PI-1  
**Estimated size:** M  
**Recommended owners:** gateway, security

**Depends on:** CTL-001, SEC-001

**Primary paths to touch:**
- `services/gateway-api`
- `crates/authn-authz`
- `crates/storage`

**Expected outputs:**
- API key auth middleware
- tenant/project context injection
- error model

**Acceptance criteria:**
- valid keys resolve tenant and project scopes
- invalid/disabled keys fail with normalized errors
- auth path emits audit and trace metadata

**Implementation notes:**
- Keep protocol parsing separate from provider adapter behavior.
- Prefer explicit compatibility errors over silent field dropping.
- Preserve provider-specific metadata in extension fields when possible.

### GWT-003 — Define protocol IR v1 with request, response, and capability models

**Phase:** PI-1  
**Estimated size:** L  
**Recommended owners:** gateway-architecture

**Depends on:** FND-001

**Primary paths to touch:**
- `crates/protocol-ir`
- `crates/core-domain`
- `docs/architecture/protocol-ir-and-protocols.md`

**Expected outputs:**
- IR data structures
- semantic validation
- provider extension fields

**Acceptance criteria:**
- IR covers text generation MVP
- unknown provider-specific metadata can be preserved
- IR versioning strategy is documented

**Implementation notes:**
- Keep protocol parsing separate from provider adapter behavior.
- Prefer explicit compatibility errors over silent field dropping.
- Preserve provider-specific metadata in extension fields when possible.

### GWT-004 — Ship OpenAI-compatible northbound chat/completions ingress

**Phase:** PI-1  
**Estimated size:** L  
**Recommended owners:** gateway

**Depends on:** GWT-001, GWT-003, PAD-001, PAD-002, RTE-002, MET-001

**Primary paths to touch:**
- `services/gateway-api`
- `crates/protocol-openai`
- `schemas/examples`

**Expected outputs:**
- OpenAI-compatible endpoint
- request parsing to IR
- response mapping from IR/provider result

**Acceptance criteria:**
- non-streaming chat requests work end-to-end
- OpenAI-compatible errors are returned
- golden fixture tests pass

**Implementation notes:**
- Keep protocol parsing separate from provider adapter behavior.
- Prefer explicit compatibility errors over silent field dropping.
- Preserve provider-specific metadata in extension fields when possible.

### GWT-005 — Implement SSE streaming relay and backpressure-safe response path

**Phase:** PI-1  
**Estimated size:** M  
**Recommended owners:** gateway

**Depends on:** GWT-004

**Primary paths to touch:**
- `services/gateway-api`
- `crates/sdk-server`
- `crates/protocol-openai`

**Expected outputs:**
- stream relay
- disconnect handling
- partial usage capture hooks

**Acceptance criteria:**
- streaming responses are flushed incrementally
- client disconnects cancel upstream requests
- stream tests cover partial and terminal events

**Implementation notes:**
- Keep protocol parsing separate from provider adapter behavior.
- Prefer explicit compatibility errors over silent field dropping.
- Preserve provider-specific metadata in extension fields when possible.

### GWT-006 — Add Anthropic-native northbound protocol support

**Phase:** PI-2  
**Estimated size:** M  
**Recommended owners:** gateway

**Depends on:** GWT-003, PAD-003, RTE-002

**Primary paths to touch:**
- `crates/protocol-anthropic`
- `services/gateway-api`

**Expected outputs:**
- Anthropic request/response mapping
- protocol-specific validation

**Acceptance criteria:**
- Anthropic-native message requests reach supported upstreams
- provider-specific fields are preserved when possible
- contract tests pass

**Implementation notes:**
- Keep protocol parsing separate from provider adapter behavior.
- Prefer explicit compatibility errors over silent field dropping.
- Preserve provider-specific metadata in extension fields when possible.

### GWT-007 — Add Gemini-native northbound protocol support

**Phase:** PI-2  
**Estimated size:** M  
**Recommended owners:** gateway

**Depends on:** GWT-003, PAD-004, RTE-002

**Primary paths to touch:**
- `crates/protocol-gemini`
- `services/gateway-api`

**Expected outputs:**
- Gemini request/response mapping
- tool/function compatibility notes

**Acceptance criteria:**
- Gemini text generation works end-to-end
- unsupported constructs fail with explicit compatibility errors
- fixtures cover mapping edge cases

**Implementation notes:**
- Keep protocol parsing separate from provider adapter behavior.
- Prefer explicit compatibility errors over silent field dropping.
- Preserve provider-specific metadata in extension fields when possible.

### GWT-008 — Build realtime session gateway skeleton

**Phase:** PI-4  
**Estimated size:** L  
**Recommended owners:** realtime

**Depends on:** FND-001, OBS-001, RTE-004

**Primary paths to touch:**
- `services/realtime-gateway`
- `crates/protocol-realtime`
- `crates/authn-authz`

**Expected outputs:**
- WebSocket service skeleton
- session auth
- realtime connection model

**Acceptance criteria:**
- service can accept authenticated sessions
- session lifecycle is traced
- no billing is coupled to the connection handshake

**Implementation notes:**
- Keep protocol parsing separate from provider adapter behavior.
- Prefer explicit compatibility errors over silent field dropping.
- Preserve provider-specific metadata in extension fields when possible.
