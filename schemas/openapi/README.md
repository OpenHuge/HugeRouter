# OpenAPI Contracts

The checked-in OpenAPI specs in this directory are generated from the Rust
contract source of truth in `crates/core-domain` and `crates/protocol-ir`.

Refresh them with:

```sh
pnpm generate
```

Current stable documents:

- `control-plane-v1.openapi.json`
- `gateway-v1.openapi.json`

Compatibility rules for `v1`:

- breaking field renames or removals require a new explicit contract version
- additive fields must stay optional until all first-party consumers tolerate them
- enum expansions are only additive when consumers handle unknown values safely
