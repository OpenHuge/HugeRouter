# Provider Adapters

[Back to Execution Workstreams](README.md)

Implement upstream provider integration through shared traits, registries, error normalization, conformance tests, and individual adapters.

## Task sequence

| Task ID | Phase | Title                                                                | Depends On       |
| ------- | ----- | -------------------------------------------------------------------- | ---------------- |
| PAD-001 | PI-1  | Define provider adapter trait model and adapter conformance test kit | GWT-003, FND-001 |
| PAD-002 | PI-1  | Implement OpenAI upstream adapter                                    | PAD-001, GWT-003 |
| PAD-003 | PI-2  | Implement Anthropic upstream adapter                                 | PAD-001, GWT-003 |
| PAD-004 | PI-2  | Implement Gemini upstream adapter                                    | PAD-001, GWT-003 |
| PAD-005 | PI-4  | Implement gateway-of-gateways adapter for upstream transit systems   | PAD-001, RTE-004 |

## Detailed tasks

### PAD-001 — Define provider adapter trait model and adapter conformance test kit

**Phase:** PI-1  
**Estimated size:** M  
**Recommended owners:** gateway-architecture

**Depends on:** GWT-003, FND-001

**Primary paths to touch:**

- `crates/provider-traits`
- `crates/testing-kit`
- `docs/architecture/provider-adapter-system.md`

**Expected outputs:**

- adapter trait set
- adapter manifest and registry model
- error normalization model
- conformance fixtures

**Acceptance criteria:**

- all adapters implement shared trait surfaces
- registry can resolve adapters by provider kind and capability profile
- adapter tests can run against mocks
- error categories map to routing decisions
- manifests can declare relay-specific compatibility details such as gateway flavor, header strategy, and sticky-session passthrough requirements
- relay compatibility profiles are extensible enough to cover One API-like, New API-like, Sub2API-like, LiteLLM-like, LMRouter-like, and generic OpenAI-compatible upstreams

**Implementation notes:**

- Normalize upstream errors into routing-relevant categories.
- Keep adapter configuration typed and region-aware.
- Adapters should pass shared conformance fixtures before integration.
- Prefer compile-time registration plus config-driven enablement before any dynamic plugin loading.

### PAD-002 — Implement OpenAI upstream adapter

**Phase:** PI-1  
**Estimated size:** M  
**Recommended owners:** provider-openai

**Depends on:** PAD-001, GWT-003

**Primary paths to touch:**

- `crates/provider-openai`

**Expected outputs:**

- OpenAI provider client
- streaming and non-streaming calls
- usage extraction hooks

**Acceptance criteria:**

- adapter passes conformance fixtures
- timeouts and retries are configurable
- provider errors map to normalized categories

**Implementation notes:**

- Normalize upstream errors into routing-relevant categories.
- Keep adapter configuration typed and region-aware.
- Adapters should pass shared conformance fixtures before integration.

### PAD-003 — Implement Anthropic upstream adapter

**Phase:** PI-2  
**Estimated size:** M  
**Recommended owners:** provider-anthropic

**Depends on:** PAD-001, GWT-003

**Primary paths to touch:**

- `crates/provider-anthropic`

**Expected outputs:**

- Anthropic provider client
- message API integration
- usage extraction hooks

**Acceptance criteria:**

- adapter passes conformance fixtures
- unsupported native features are surfaced clearly
- timeout and region configs are supported

**Implementation notes:**

- Normalize upstream errors into routing-relevant categories.
- Keep adapter configuration typed and region-aware.
- Adapters should pass shared conformance fixtures before integration.

### PAD-004 — Implement Gemini upstream adapter

**Phase:** PI-2  
**Estimated size:** M  
**Recommended owners:** provider-gemini

**Depends on:** PAD-001, GWT-003

**Primary paths to touch:**

- `crates/provider-gemini`

**Expected outputs:**

- Gemini provider client
- compatibility layer
- usage extraction hooks

**Acceptance criteria:**

- adapter passes conformance fixtures
- model capability metadata is discoverable
- error normalization is documented

**Implementation notes:**

- Normalize upstream errors into routing-relevant categories.
- Keep adapter configuration typed and region-aware.
- Adapters should pass shared conformance fixtures before integration.

### PAD-005 — Implement gateway-of-gateways adapter for upstream transit systems

**Phase:** PI-4  
**Estimated size:** M  
**Recommended owners:** provider-gateway

**Depends on:** PAD-001, RTE-004

**Primary paths to touch:**

- `crates/provider-gateway`

**Expected outputs:**

- adapter for upstream gateway providers
- health and capability metadata model
- compatibility profiles for common relay and broker families

**Acceptance criteria:**

- adapter can call an upstream OpenAI-compatible gateway
- upstream gateway errors preserve diagnostic detail
- routing engine can score it alongside native providers
- adapter can authenticate against third-party relay keys and preserve declared relay-required headers without leaking them into unrelated providers
- profile coverage includes at least generic OpenAI-compatible gateways plus documented family-specific handling for One API-like, New API-like, Sub2API-like, and broker-style gateways such as LiteLLM or LMRouter where needed

**Implementation notes:**

- Normalize upstream errors into routing-relevant categories.
- Keep adapter configuration typed and region-aware.
- Adapters should pass shared conformance fixtures before integration.
- Treat relay-family compatibility as typed profiles rather than site-by-site code forks.
