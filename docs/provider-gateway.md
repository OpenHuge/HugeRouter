# Provider Gateway Transit Adapter

`PAD-005` v1 adds a first-class `provider_id="gateway"` target that treats an upstream OpenAI-compatible gateway as a governed transit provider inside HugeRouter's normal routing chain.

## Why This Is Not Just an `openai` Alias

An upstream gateway is not equivalent to a native OpenAI target:

- It adds an extra transit hop, so routing should score it with explicit hop-aware latency and trust adjustments.
- It has different failure semantics. HugeRouter must preserve upstream gateway diagnostics while still returning HugeRouter-normalized errors.
- It requires loop protection. Native OpenAI targets do not need self-target detection or transit-header recursion guards.
- It needs its own header policy. A gateway target may safely receive only a narrow allowlist plus HugeRouter-authored transit headers.
- It can have route-scoped transit cost semantics that differ from a native provider's flat price assumptions.

Treating it as a plain `openai` alias would hide those differences and would force loop prevention, provenance, and transit metadata into ad hoc handler branches.

## Transit Semantics

`provider_id="gateway"` means:

- HugeRouter normalizes northbound requests first.
- The selected target is still ranked alongside native providers.
- The adapter then forwards the normalized request to the upstream gateway's `/chat/completions` endpoint using OpenAI-compatible JSON.
- Error diagnostics from the upstream gateway are retained in normalized error details, including `upstream_status_code`, `upstream_code`, and request-id style gateway diagnostics when present.

The current v1 transit metadata is runtime-typed and includes:

- `gateway_kind=openai_compatible`
- `gateway_name=<upstream host>`
- `route_cost_scope=<route_policy.protocol_family>`
- `transit_hops=1`
- `preserves_error_diagnostics=true`

## Loop Protection

HugeRouter rejects transit execution when either condition is true:

- The incoming request already contains HugeRouter transit control headers:
  - `x-hugerouter-transit-hop`
  - `x-hugerouter-transit-via`
  - `x-hugerouter-transit-origin`
  - `x-hugerouter-transit-provider`
- The upstream target endpoint matches HugeRouter's own public origin.

When this triggers, HugeRouter returns the normalized error code `transit_loop_detected`.

Set `GATEWAY_PUBLIC_BASE_URL` so self-target detection can compare the configured upstream gateway endpoint against the current HugeRouter deployment origin.

## Header Policy

Allowed passthrough headers:

- `accept`
- `accept-encoding`
- `accept-language`
- `idempotency-key`
- `openai-organization`
- `openai-project`
- `user-agent`
- `x-correlation-id`
- `x-request-id`
- `x-trace-id`

Headers stripped from caller input before transit forwarding:

- `authorization`
- `connection`
- `content-length`
- `forwarded`
- `host`
- `proxy-authorization`
- `proxy-connection`
- `te`
- `trailer`
- `transfer-encoding`
- `upgrade`
- `via`
- `x-forwarded-for`
- `x-forwarded-host`
- `x-forwarded-proto`
- `x-portkey-forward-headers`
- All `x-hugerouter-transit-*` control headers

Headers authored or rewritten by HugeRouter:

- `authorization: Bearer <target api key>`
- `content-type: application/json`
- `x-request-id: <HugeRouter request id>`
- `x-trace-id: <HugeRouter trace id>`
- `x-hugerouter-transit-hop: 1`
- `x-hugerouter-transit-via: gateway-api`
- `x-hugerouter-transit-origin: <HugeRouter public origin or "unknown">`
- `x-hugerouter-transit-provider: <provider_resource_id>`

## Configuration

Shared transit env vars:

- `GATEWAY_TRANSIT_API_KEY`
- `GATEWAY_TRANSIT_MODEL` (optional override; default is to preserve the route's model alias)
- `GATEWAY_PUBLIC_BASE_URL`
- `GATEWAY_TRANSIT_OPENAI_USD_PER_1K_TOKENS`
- `GATEWAY_TRANSIT_ANTHROPIC_USD_PER_1K_TOKENS`
- `GATEWAY_TRANSIT_GEMINI_USD_PER_1K_TOKENS`

Per-target overrides:

- `PROVIDER_RESOURCE_<PROVIDER_RESOURCE_ID>_API_KEY`
- `PROVIDER_RESOURCE_<PROVIDER_RESOURCE_ID>_MODEL`

`<PROVIDER_RESOURCE_ID>` is uppercased and normalized to `[A-Z0-9_]`.

## Validation

Run:

```bash
cargo test -p provider-traits -p provider-gateway -p gateway-api
cargo test
```

The targeted tests cover:

- successful transit forwarding
- upstream `4xx` / `5xx`
- timeout handling
- transit header loop prevention
- route scoring selecting a transit provider
- normalized loop error mapping
