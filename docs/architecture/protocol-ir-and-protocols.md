# 9. Internal Representation for AI Traffic

[Back to Docs Index](../README.md)

The system must define a protocol-independent internal representation called **IR**.

The IR is the canonical semantic form used after parsing northbound requests and before rendering southbound upstream requests.

### 9.0 Current Implementation Reality

The repository should currently be described this way:

- `implemented`: one OpenAI-compatible synchronous northbound path and one real OpenAI adapter path
- `bootstrap-only`: contracts, schemas, and typed shared packages are substantially ahead of runtime breadth
- `planned`: Responses-native canonicalization in runtime, MCP execution, A2A execution, realtime execution, and typed protocol-resource registration across all protocol families

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
- a2a_context (Agent-to-Agent task delegation, agent card metadata)
- tool_definitions
- tool_choice_mode
- response_mode
- streaming_mode
- session_affinity_key
- ephemeral_auth_context (for WebRTC/Voice)
- agent_identity (non-human identity for agentic callers)
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
- A2A (Agent-to-Agent) task delegation and coordination
- JSON schema constrained output
- reasoning mode
- cached input billing
- realtime session (WebRTC / WebSockets)
- function/tool streaming
- long context
- batch execution
- semantic cache eligible

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

### 9.6 OpenAI Family Canonicalization Policy

As of **April 20, 2026**, OpenAI recommends the Responses API for new projects, and the response shape is item-oriented rather than strictly message-oriented.

Design rule:

- the IR should treat typed items, tool activity, and response metadata as the canonical OpenAI-family semantics
- Chat Completions compatibility should be implemented as a translation surface, not as the canonical internal shape
- routing, policy, metering, audit, and diagnostics should not depend on `choices[0].message` assumptions
- if a request enters via Chat Completions, the protocol layer should normalize it into the same semantic units that a Responses request would produce where possible

This keeps the gateway aligned with the long-term OpenAI direction while still allowing compatibility ingress surfaces.

### 9.7 Typed Protocol and Route Resource Model

Protocol support should be expressed as typed resources rather than handler-local flags.

The architecture should distinguish at least these resource types:

- `NorthboundProtocolSurface`: one ingress compatibility surface such as Chat Completions, Responses, MCP, or another future protocol family
- `RouteResource`: one declarative route object that maps intent and policy to candidate target classes
- `ProviderResource`: one concrete upstream execution target or account-backed endpoint
- `AdapterManifest`: one runtime-declared execution capability and lifecycle contract
- `McpResource`: one governed MCP server or MCP capability endpoint
- planned extensions such as `A2aResource` should become first-class only after the published protocol/resource contracts add them explicitly

Minimum typed metadata expected on every resource:

- stable identifier
- schema version
- protocol family or lifecycle family
- capability declarations
- auth mode declarations
- residency or regional attributes
- observability labels safe for diagnostics
- stability level such as `stable`, `beta`, or `experimental`

Boundary rule:

- northbound compatibility surfaces describe ingress semantics
- route resources describe control-plane intent
- provider, MCP, and A2A resources describe governed remote execution surfaces
- manifests describe what runtime code can actually execute

No one resource type should absorb the responsibilities of the others.

Current contract reminder:

- the published protocol-family contracts currently enumerate `openai_chat`, `openai_responses`, `mcp_streamable_http`, and `realtime_webrtc`
- the current control-plane contract surface publishes `RoutePolicy`, not a richer typed `RouteResource`
- A2A remains planned in the architecture, but it is not yet part of the generated protocol-family enum or typed route-resource contracts in this repository

---

## 10. Protocol Support

### 10.1 Northbound Protocols

The system should support at least the following northbound protocol families:

1. OpenAI Chat Completions compatible
2. OpenAI Responses API compatible
3. Anthropic Messages compatible
4. Gemini native request forms
5. Realtime event streams (OpenAI Realtime API GA over WebSockets/WebRTC)
6. Model Context Protocol (MCP) for agentic orchestrations via Streamable HTTP
7. Agent-to-Agent (A2A) protocol for multi-agent task delegation and coordination
8. Internal SDK-native protocol for power users, optional

### 10.2 Southbound Protocols

The system should support at least the following southbound target types:

1. Native OpenAI upstream (including GA ephemeral key generation `POST /v1/realtime/client_secrets` and WebRTC via `/v1/realtime/calls`)
2. Native Anthropic upstream
3. Native Gemini upstream
4. Gateway-style upstreams using OpenAI-compatible APIs
5. OAuth-backed adapters for specialized upstream resources
6. Realtime socket or bidirectional channel upstreams (WebSockets/WebRTC)
7. MCP Server endpoints for tool resolution via Streamable HTTP
8. A2A-compliant agent endpoints discovered via Agent Cards (`/.well-known/agent-card.json`)

### 10.3 Versioning Strategy

Protocol packages must expose versioned parsers and serializers. Breaking protocol changes must not force the entire gateway to upgrade in lockstep without compatibility shims.

### 10.4 Composable Protocol Handling

Protocol support should be assembled through registries and manifests rather than switch-heavy service code.

The gateway should be able to:

- register protocol handlers at bootstrap
- resolve a parser/serializer pair by protocol family and version
- expose handler capability metadata to diagnostics and control plane APIs
- add new protocol handlers without rewriting unrelated ingress paths

Handler and resource rule:

- protocol handlers should resolve from typed protocol-surface metadata and typed route resources, not from switch-heavy request handlers
- a route resource should be able to state which protocol families it accepts northbound and which target lifecycle families it may dispatch southbound

### 10.5 MCP Transport Direction

As of **April 20, 2026**, the recommended MCP direction is:

- implement **Streamable HTTP** as the first-class MCP transport
- keep legacy HTTP + SSE support only as an explicit compatibility layer if needed for older clients
- model MCP auth around OAuth 2.1-compatible flows with PKCE, audience binding, and RFC 8414 server metadata discovery
- design for **stateless transport** evolution as the MCP specification roadmap is actively moving toward stateless sessions to improve compatibility with load balancers and horizontal scaling

This keeps the gateway aligned with the current MCP specification direction while avoiding permanent investment in already-deprecated transport assumptions.

### 10.6 A2A Protocol Direction

As of **April 20, 2026**, the A2A protocol has been standardized under the Linux Foundation and should be treated as a production-ready coordination layer:

- implement **Agent Card** discovery via `/.well-known/agent-card.json` for public capability advertisement and resolution
- model authenticated **extended Agent Card** retrieval as a distinct follow-up step, not as something the public card should leak by default
- support A2A **Task lifecycle** states at least through `submitted`, `working`, `input-required`, `auth-required`, `completed`, `canceled`, `failed`, and `rejected`
- preserve A2A task immutability: once a task reaches a terminal state, follow-up work should create a new task in the same context instead of mutating the old unit of work
- model A2A auth around the agent card's declared auth methods (OAuth 2.0, API keys, or mTLS)
- adopt the **Hub-and-Spoke Gateway** model where the gateway validates, governs, and audits inter-agent traffic rather than allowing uncontrolled peer-to-peer agent communication
- support asynchronous task updates via webhooks and real-time streaming via SSE
- treat Agent Card signing and verification as a future hardening path worth preserving structurally, even if signature verification is not required in the first slice

A2A complements MCP: MCP handles vertical integration (agent-to-tools/data), A2A handles horizontal coordination (agent-to-agent). The gateway should govern both.

### 10.7 OpenAI Realtime API GA Migration

As of **April 20, 2026**, the published OpenAI deprecation schedule lists the Realtime beta interface (`OpenAI-Beta: realtime=v1`) for removal on **February 27, 2026**. The gateway should therefore treat the GA interface as the only supported OpenAI Realtime target:

- ephemeral keys via `POST /v1/realtime/client_secrets` (GA endpoint)
- WebRTC initialization via `/v1/realtime/calls` (GA endpoint)
- GA event naming (e.g., `response.output_text.delta` replaces beta `response.text.delta`)
- remove `OpenAI-Beta: realtime=v1` header from adapter implementations
- prefer browser/client flows over WebRTC and middle-tier server flows over WebSocket, matching the current official guidance for connection modes

### 10.8 Protocol Governance Boundary

To avoid architecture drift, the gateway should treat the major protocol families as different kinds of responsibility:

- OpenAI Responses and Chat Completions compatibility are northbound application request surfaces
- MCP is a capability and context transport, usually suited to tool/data interactions and simpler request-response patterns
- A2A is a collaboration transport for stateful, multi-step, and multi-agent task execution

Guardrail:

- do not collapse MCP servers and A2A agents into one generic "remote thing" abstraction too early
- do not expose every MCP tool as an A2A agent or every A2A skill as an MCP tool by default
- only introduce bridging when a concrete product path requires it, and record the lossiness or lifecycle mismatch explicitly

### 10.9 Development Scope Policy

For the first implementation:

- the IR should be rich enough to support OpenAI Responses-style semantics even if Chat Completions compatibility is delivered first
- MCP should be represented as a first-class protocol family, not as opaque passthrough JSON
- A2A should be modeled as an explicit protocol family with Agent Card discovery, task lifecycle, and governance hooks
- realtime, MCP, and A2A transports should share lifecycle, auth, and trace abstractions where possible instead of forking separate gateway stacks too early

Implementation-state reminder:

- the runtime may ship Chat Completions compatibility first
- the canonical IR should still be designed around richer Responses-era semantics
- MCP, A2A, and realtime should remain `planned` until there is real typed registration, real execution wiring, and real receipt or usage semantics in the running services

---
