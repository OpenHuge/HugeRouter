# AWS Bedrock Provider Adapter

[Back to Docs Index](README.md)

`provider_id="bedrock"` routes a HugeRouter text chat request to Amazon Bedrock through the native `bedrock-runtime` Converse API.

## Runtime Semantics

- The adapter uses the official AWS SDK for Rust, so credentials, SigV4 signing, retries, and region handling come from the AWS default credential chain.
- HugeRouter does not pass an API key header to Bedrock. The `api_key` field on the generic provider endpoint is intentionally ignored for this adapter.
- The selected provider resource region is passed to the SDK client. If no region is present, the adapter falls back to `AWS_REGION`, `AWS_DEFAULT_REGION`, then `us-east-1`.
- `endpoint_base_url` is treated as an endpoint override when present. `GATEWAY_BEDROCK_ENDPOINT_URL` takes precedence for local or custom endpoint testing.

## Control-plane Provider Resource

Bedrock is represented as a normal provider resource with `provider_id="bedrock"`. The generic `auth_kind` remains `api_key` for schema compatibility, but the runtime adapter ignores the provider API key and resolves AWS credentials through the AWS SDK credential chain.

```json
{
  "provider_resource_id": "prvrsrc_bedrock_claude",
  "tenant_id": "tenant_acme",
  "project_id": "proj_acme_ops",
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

Operational seed data includes a Bedrock route policy for `proj_acme_ops` that requires only `chat_completions`, matching the first adapter's non-streaming Converse text scope.

## Model And Inference Configuration

Model ID precedence:

1. `PROVIDER_RESOURCE_<PROVIDER_RESOURCE_ID>_MODEL`
2. `GATEWAY_BEDROCK_MODEL`
3. the route request model alias

Adapter-level options:

- `GATEWAY_BEDROCK_MAX_TOKENS` defaults to `1024`
- `GATEWAY_BEDROCK_TEMPERATURE` is optional and only sent when configured
- `GATEWAY_BEDROCK_USD_PER_1K_TOKENS` controls local cost estimation

Optional guardrail configuration:

- `GATEWAY_BEDROCK_GUARDRAIL_ID`
- `GATEWAY_BEDROCK_GUARDRAIL_VERSION`
- `GATEWAY_BEDROCK_GUARDRAIL_TRACE`

## Supported Scope

This first adapter supports non-streaming Converse text messages. `ConverseStream`, model-specific `InvokeModel` bodies, OpenAI-compatible `bedrock-mantle` endpoints, and multimodal content are intentionally deferred.

## Pricing Catalog

The built-in metering catalog includes a provider-level `bedrock` rate card so ledger quotes do not fall back to the default provider rates. It is intentionally provider-level, not model-level; tenants that need exact model, region, service tier, or committed-throughput pricing should override the catalog through contract or tenant-specific pricing.

## Validation

Run:

```bash
cargo test -p provider-bedrock -p provider-traits -p gateway-api
pnpm rust:check
```

The tests cover request shaping, usage extraction, error normalization, registry registration, and region propagation into the provider execution context.
