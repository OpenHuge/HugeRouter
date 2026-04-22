# 12. Provider Adapter System

[Back to Docs Index](../README.md)

The provider layer should be explicitly designed for **pluggability** and **composition**. Adapters are not special-case branches inside the gateway. They are modules discovered through a registry, selected by capability and policy, and executed inside a composed runtime pipeline.

### 12.0 Current Implementation Reality

The repository should currently be described this way:

- `implemented`: one adapter trait and registry model plus one real OpenAI adapter
- `bootstrap-only`: much of the broader provider, control-plane discovery, and diagnostics integration is specified but not yet runtime-complete
- `planned`: MCP adapters, A2A adapters, realtime bidirectional adapters, manifest exposure to the control plane, and compatibility gating against snapshot versions

### 12.1 Adapter Responsibilities

Each provider adapter is responsible for:

- mapping IR to the upstream protocol
- injecting upstream auth and endpoint configuration
- handling stateless HTTP translation AND stateful bidirectional WebSocket (`ws`/`wss`) streaming
- in-flight frame inspection for WebSockets to monitor events and safety without terminating the connection
- progressive usage extraction (emitting partial token usage during long-lived sessions)
- provider-specific error normalization
- provider-specific idempotency handling
- provider metadata enrichment
- declaring capabilities, configuration schema, and operational traits

If the target is not a plain model provider, the adapter must also make its lifecycle model explicit:

- MCP-oriented adapters expose tool and capability interaction semantics
- A2A-oriented adapters expose task submission, continuation, and terminal-state semantics
- realtime adapters expose session setup, control, and teardown semantics

### 12.2 Design Principle

The first implementation should optimize for:

- stable extension contracts
- explicit registration
- typed manifests
- capability-driven selection
- middleware composition around adapter execution

The first implementation should **not** require runtime dynamic library loading. We can stay pluggable with compile-time modules plus config-driven registration and add WASM or ABI-stable plugins later only if the product really needs it.

### 12.3 Adapter Contract Shape

Conceptually, provider adapters implement a contract similar to:

```text
trait ProviderAdapter {
  fn manifest(&self) -> AdapterManifest;
  fn supported_capabilities(&self) -> CapabilityProfile;
  fn configuration_schema(&self) -> AdapterConfigSchema;

  // Stateless execution
  async fn execute(&self, request: AiRequestIr, ctx: ExecutionContext) -> AdapterResult;

  // Stateful bidirectional execution (for WebSockets / WebRTC)
  async fn execute_bidirectional(&self, stream: BidirectionalStream, ctx: ExecutionContext) -> Result<(), GatewayError>;

  fn normalize_error(&self, error: UpstreamError) -> GatewayError;
  fn extract_usage(&self, response: &AdapterResponse) -> UsageSummary;
}

trait AdapterFactory {
  fn adapter_kind(&self) -> AdapterKind;
  fn build(&self, config: AdapterConfig, deps: AdapterDependencies) -> Arc<dyn ProviderAdapter>;
}
```

### 12.4 Required Manifest Fields

Each adapter manifest should declare at least:

- adapter identifier
- adapter kind
- provider kind
- lifecycle family such as `inference`, `mcp`, `a2a`, or `realtime`
- supported protocols
- capability profile
- region or residency constraints
- auth modes
- upstream provenance or trust class support
- retry and timeout characteristics
- observability tags
- stability level such as `stable`, `beta`, or `experimental`
- manifest schema version
- supported routing hints such as latency-aware, cost-aware, or priority-aware selection
- compatibility range for config snapshots and the current route-policy contract surface, with future extension to typed route resources once those contracts exist

### 12.4.1 Manifest Boundary Rule

Adapter manifests should be the runtime truth for what executable code supports.

Control-plane resources should not assume an adapter can do something unless the loaded manifest declares it. Conversely, the runtime should not accept a provider resource or effective route-policy snapshot whose required capability, lifecycle family, or schema version falls outside the manifest compatibility contract. If typed route resources are added later, they should be held to the same boundary instead of bypassing manifest checks.

### 12.5 Registry Model

Adapters should be loaded into an in-process registry during service bootstrap.

The registry is responsible for:

- registering adapter factories
- resolving adapters by provider kind, capability, and runtime config
- exposing manifest metadata to the control plane and diagnostics surfaces
- preventing duplicate adapter IDs or conflicting capability declarations

Conceptually:

```text
AdapterRegistry
- register(factory)
- resolve(provider_kind, capability_requirements, tenant_constraints)
- list_manifests()
- get_manifest(adapter_id)
```

### 12.6 Execution Pipeline Composition

Adapter execution should be wrapped by composable middleware stages rather than embedded directly in service handlers.

Suggested pipeline order:

1. request shaping and trace decoration
2. policy enforcement
3. route selection
4. adapter resolution from registry
5. timeout / retry / circuit-breaker middleware
6. adapter execute
7. usage extraction
8. response normalization
9. telemetry and audit emission

This makes adapter behavior composable without duplicating cross-cutting logic in every provider crate.

Inspired by Envoy-style filter discipline, this pipeline order is an architectural contract. If a later stage can alter route or target resolution after policy has already been enforced, the system must either prohibit that mutation or force explicit re-evaluation and audit capture.

Execution-state rule:

- route strategy and admission outcomes must remain visible outside the adapter
- adapters may emit hints and normalized errors, but they must not silently redefine routing policy or budget policy

### 12.6.1 Routing Strategy Boundary

Borrowing the useful parts of LiteLLM's router model, routing strategy should be explicit but remain outside the adapter implementation.

Examples of strategy families we should support over time:

- static priority
- least-busy
- latency-aware
- cost-aware
- usage- or quota-aware

Important boundary rule:

- adapters expose capabilities and operational traits
- routing engines choose deployments and fallback order
- adapters must not quietly implement their own hidden traffic-selection policy

### 12.7 Adapter Categories

- native API adapters
- gateway adapters
- account-pool adapters
- OAuth-backed adapters
- realtime adapters
- A2A agent adapters (routing tasks to external agents discovered via Agent Cards)
- semantic cache adapters (checking and populating semantic similarity cache before upstream dispatch)
- experimental adapters

These categories should be represented as metadata, not hard-coded branches. The routing and diagnostics layers should reason from manifest and capability descriptors rather than from crate names.

Protocol-boundary guardrail:

- do not collapse provider, MCP server, and A2A agent integrations into one generic "remote endpoint" abstraction too early
- adapters may share plumbing, but manifests must still declare whether they execute prompt/response work, tool/context work, or task/delegation work
- bridge behavior between those categories should be explicit and reviewable because lifecycle and auth semantics differ materially

### 12.8 Isolation Rules

Adapters should be isolated at code and configuration level. A provider-specific failure must not cascade into unrelated providers.

Isolation requirements:

- per-adapter config validation
- per-adapter timeout and retry envelopes
- normalized error categories for routing decisions
- adapter-local cooldown or quarantine state surfaced back to routing, not buried inside provider code
- no adapter may mutate shared request state outside its execution context
- feature flags or experimental adapters must be disableable at registration time

### 12.9 Testing Strategy

Every adapter should pass a shared conformance suite that validates:

- manifest correctness
- capability declarations
- request translation
- response normalization
- streaming behavior
- usage extraction
- retryable vs non-retryable error classification

### 12.10 Control Plane Integration

The control plane should treat adapters as discoverable runtime modules rather than opaque strings.

It should be possible to:

- list available adapter manifests
- validate whether a provider resource matches an adapter config schema
- preview capability compatibility before saving route definitions
- surface adapter stability and operational constraints in diagnostics

### 12.11 Compatibility Contract

The runtime should validate compatibility across four layers:

- current route-policy contract version, with future extension to typed route-resource versions once they exist
- provider resource schema version
- adapter manifest schema version
- running binary compatibility version

If these layers do not overlap safely, the control plane should refuse activation and the data plane should continue using the last-known-good compatible snapshot rather than partially activating incompatible config.

---
