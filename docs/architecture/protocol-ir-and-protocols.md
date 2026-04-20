# 9. Internal Representation for AI Traffic

[Back to Docs Index](../README.md)

The system must define a protocol-independent internal representation called **IR**.

The IR is the canonical semantic form used after parsing northbound requests and before rendering southbound upstream requests.

### 9.1 Why an IR Exists

Without an IR, protocol translations become brittle and lossy. The IR allows the platform to:

- reason about capability requirements
- apply policy before upstream selection
- route by semantics rather than string model names
- preserve structured metadata such as tool calls and streaming modes
- support multi-protocol ingress and egress

### 9.2 IR Core Fields

```text
AiRequestIr
- request_id
- tenant_id
- project_id
- protocol_family
- requested_model_alias
- required_capabilities
- optional_capabilities
- input_messages
- input_modalities
- output_modalities
- mcp_context (Model Context Protocol references)
- tool_definitions
- tool_choice_mode
- response_mode
- streaming_mode
- session_affinity_key
- ephemeral_auth_context (for WebRTC/Voice)
- metadata
- security_context
- billing_context
- trace_context
- extension_fields
```

### 9.3 Capability Examples

- text input
- text output
- image input
- image generation
- audio input / voice output (Realtime)
- tool calling
- MCP (Model Context Protocol) integration
- JSON schema constrained output
- reasoning mode
- cached input billing
- realtime session (WebRTC / WebSockets)
- function/tool streaming
- long context
- batch execution

### 9.4 Response IR

A similar response-side IR should represent:

- text deltas
- content blocks
- tool call requests
- partial usage updates
- final usage summary
- safety / policy flags
- provider annotations
- extension_fields

### 9.5 Extensibility Rules

The IR should be extensible without turning into an untyped dump.

Rules:

- core fields remain strongly typed and versioned
- protocol- or provider-specific extensions live in explicit `extension_fields`
- extension fields must be namespaced by plugin, protocol family, or provider kind
- routing and policy code must not depend on arbitrary extensions unless they are elevated into first-class capability contracts
- unknown extensions should survive round-trips where safe, but they must not silently alter security-critical behavior


---


## 10. Protocol Support

### 10.1 Northbound Protocols

The system should support at least the following northbound protocol families:

1. OpenAI Chat Completions compatible
2. OpenAI Responses API compatible
3. Anthropic Messages compatible
4. Gemini native request forms
5. Realtime event streams (OpenAI Realtime API over WebSockets/WebRTC)
6. Model Context Protocol (MCP) for agentic orchestrations
7. Internal SDK-native protocol for power users, optional

### 10.2 Southbound Protocols

The system should support at least the following southbound target types:

1. Native OpenAI upstream (including ephemeral key generation `POST /v1/realtime/client_secrets`)
2. Native Anthropic upstream
3. Native Gemini upstream
4. Gateway-style upstreams using OpenAI-compatible APIs
5. OAuth-backed adapters for specialized upstream resources
6. Realtime socket or bidirectional channel upstreams (WebSockets/WebRTC)
7. MCP Server endpoints for tool resolution

### 10.3 Versioning Strategy

Protocol packages must expose versioned parsers and serializers. Breaking protocol changes must not force the entire gateway to upgrade in lockstep without compatibility shims.

### 10.4 Composable Protocol Handling

Protocol support should be assembled through registries and manifests rather than switch-heavy service code.

The gateway should be able to:

- register protocol handlers at bootstrap
- resolve a parser/serializer pair by protocol family and version
- expose handler capability metadata to diagnostics and control plane APIs
- add new protocol handlers without rewriting unrelated ingress paths

### 10.5 MCP Transport Direction

As of **April 20, 2026**, the recommended MCP direction is:

- implement **Streamable HTTP** as the first-class MCP transport
- keep legacy HTTP + SSE support only as an explicit compatibility layer if needed for older clients
- model MCP auth around OAuth 2.1-compatible flows with PKCE, audience binding, and discovery-friendly server metadata

This keeps the gateway aligned with the current MCP specification direction while avoiding permanent investment in already-deprecated transport assumptions.

### 10.6 Development Scope Policy

For the first implementation:

- the IR should be rich enough to support OpenAI Responses-style semantics even if Chat Completions compatibility is delivered first
- MCP should be represented as a first-class protocol family, not as opaque passthrough JSON
- realtime and MCP transports should share lifecycle, auth, and trace abstractions where possible instead of forking separate gateway stacks too early

---
