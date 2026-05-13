# PI-4 — Ecosystem and Realtime

[Back to Task Catalog Index](README.md)

Advanced ecosystem and realtime tasks.

## Recommended execution order

- **GWT-008** — Build realtime session gateway skeleton _(depends on: FND-001, OBS-001, RTE-004)_
- **PAD-005** — Implement gateway-of-gateways adapter for upstream transit systems _(depends on: PAD-001, RTE-004)_

## Detailed tasks

### GWT-008 — Build realtime session gateway skeleton

**Stream:** Gateway and Protocol Ingress  
**Owners:** realtime  
**Depends on:** FND-001, OBS-001, RTE-004

**Paths:**

- `services/realtime-gateway`
- `crates/protocol-realtime`
- `crates/authn-authz`

**Outputs:**

- WebSocket service skeleton
- session auth
- realtime connection model

**Acceptance criteria:**

- service can accept authenticated sessions
- session lifecycle is traced
- no billing is coupled to the connection handshake

### PAD-005 — Implement gateway-of-gateways adapter for upstream transit systems

**Stream:** Provider Adapters  
**Owners:** provider-gateway  
**Depends on:** PAD-001, RTE-004

**Paths:**

- `crates/provider-gateway`

**Outputs:**

- adapter for upstream gateway providers
- health and capability metadata model
- compatibility profiles for common relay and broker families

**Acceptance criteria:**

- adapter can call an upstream OpenAI-compatible gateway
- upstream gateway errors preserve diagnostic detail
- routing engine can score it alongside native providers
- adapter can authenticate against third-party relay keys and preserve declared relay-required headers without leaking them into unrelated providers
- profile coverage includes at least generic OpenAI-compatible gateways plus documented family-specific handling for One API-like, New API-like, Sub2API-like, LiteLLM-like, and LMRouter-like upstreams where needed
