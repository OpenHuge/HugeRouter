# AWS Bedrock Provider Adapter

[Back to Docs Index](README.md)

`provider_id="bedrock"` routes a HugeRouter text chat request to Amazon Bedrock through the native `bedrock-runtime` Converse API.

## Runtime Semantics

- The adapter uses the official AWS SDK for Rust, so credentials, SigV4 signing, retries, and region handling come from the AWS default credential chain.
- HugeRouter does not pass an API key header to Bedrock. The `api_key` field on the generic provider endpoint is intentionally ignored for this adapter.
- The selected provider resource region is passed to the SDK client. If no region is present, the adapter falls back to `AWS_REGION`, `AWS_DEFAULT_REGION`, then `us-east-1`.
- `endpoint_base_url` is treated as an endpoint override when present. `GATEWAY_BEDROCK_ENDPOINT_URL` takes precedence for local or custom endpoint testing.

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

## Validation

Run:

```bash
cargo test -p provider-bedrock -p provider-traits -p gateway-api
pnpm rust:check
```

The tests cover request shaping, usage extraction, error normalization, registry registration, and region propagation into the provider execution context.
