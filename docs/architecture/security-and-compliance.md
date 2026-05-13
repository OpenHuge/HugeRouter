# 19. Security Requirements

[Back to Docs Index](../README.md)

### 19.1 Network Segmentation

The system should expose at least:

- public API ingress
- private control plane ingress
- internal service mesh/network
- private datastore network

### 19.2 Secret Management

- upstream API keys and OAuth tokens must never be stored in plaintext without encryption
- admin session secrets must be centrally managed
- secret rotation must be automatable

### 19.2.1 Upstream Credential Provenance

Not all upstream credentials are equivalent from a legal, operational, or trust perspective.

Every provider resource should record:

- credential owner model such as platform-managed, partner-managed, or BYO customer key
- provenance class such as official API, official gateway, brokered pool, or unofficial client channel
- contractual or policy eligibility for each deployment tier

Reverse-engineered or otherwise unofficial client channels should be treated as high-risk and disabled by default in production configurations.

### 19.3 Data Protection

- TLS in transit
- encryption at rest for databases and object stores
- selective field encryption for especially sensitive data
- audit logs for all sensitive configuration changes

### 19.4 AI & LLM Security Guardrails (OWASP Top 10 for LLM Applications 2025)

The gateway acts as the primary defense layer against LLM application vulnerabilities:

- **Prompt Injection (LLM01):** In-flight inspection and sanitization of prompts/responses to prevent instruction overrides.
- **Sensitive Information Disclosure (LLM02):** Real-time PII redaction rules applied before traffic hits external upstream providers.
- **Supply Chain (LLM03):** Validation of third-party model and plugin provenance.
- **Data and Model Poisoning (LLM04):** Integrity checks for embedding data in semantic cache pipelines.
- **Improper Output Handling (LLM05):** Response validation before downstream consumption.
- **Excessive Agency (LLM06):** Strict validation of MCP contexts and tool calls against tenant-defined access control policies.
- **System Prompt Leakage (LLM07):** Response filtering to prevent system prompt extraction.
- **Vector and Embedding Weaknesses (LLM08):** Integrity controls for semantic cache vector stores.
- **Unbounded Consumption (LLM10):** Real-time progressive metering and hierarchical quota enforcement.

### 19.5 Agentic Security Guardrails (OWASP Top 10 for Agentic Applications 2026)

As autonomous agents operate with their own identity and credentials, the gateway must enforce agent-specific governance:

- **Goal Hijacking:** Detection and prevention of adversaries tricking agents into pursuing unauthorized objectives. The gateway should validate that agent task delegation chains remain within authorized scope.
- **Tool Misuse and Exploitation:** Strict MCP tool allowlists and scope limits per tenant, project, and agent identity. A2A task delegation must not bypass tool governance.
- **Identity and Privilege Abuse:** Non-Human Identity (NHI) management for agent credentials. Agent keys must carry explicit scope, budget, and capability restrictions.
- **Cascading Failures:** Circuit breakers and blast-radius limits for agent-initiated multi-step workflows. The gateway should detect and terminate recursive agent loops.
- **Memory Poisoning:** Validation and integrity checking for agent context and memory state passed through MCP or A2A channels.
- **Unexpected Code Execution:** Sandboxing and policy enforcement for agent-generated code execution requests.
- **Denial of Wallet & Runaway Loops:** Real-time progressive metering with session kill-switches for unbounded agentic consumption.
- **Opaque Upstream Brokerage:** Requests must not be silently routed through unofficial resale paths or hidden client-channel emulation without explicit policy allowance and operator visibility.

### 19.6 A2A Protocol Security

Agent-to-Agent communication introduces additional security requirements:

- Agent Card validation before accepting delegation requests from remote agents
- public Agent Card and authenticated extended Agent Card data must be distinguished so private capability details are not leaked by discovery alone
- Agent Card signing and signature verification should be preserved as a future hardening path even if not required in the first slice
- Authentication method enforcement as declared in agent cards (OAuth 2.0, API keys, or mTLS)
- Hub-and-Spoke enforcement: direct peer-to-peer agent communication should be denied by default in production; all inter-agent traffic should transit through the gateway for governance
- A2A task scope validation: delegated tasks must not exceed the delegating agent's own permission boundaries
- Agent identity chain tracking for multi-hop delegations
- terminal A2A tasks should be immutable; follow-up work should create a new task rather than mutating completed or failed history

### 19.6.1 MCP Transport Security

Streamable HTTP MCP integrations introduce transport-specific security requirements:

- validate the `Origin` header for browser-based clients
- prefer binding local development servers to `localhost` unless explicit remote exposure is intended
- treat session identifiers as bearer-capable secrets and avoid logging or reflecting them casually
- keep OAuth metadata, audience binding, and redirect validation aligned with RFC 8414-style discovery and OAuth 2.1 expectations

Guardrail:

- do not treat MCP as "just JSON over HTTP"; the transport and auth semantics materially affect browser safety and delegated tool access

### 19.7 Non-Human Identity (NHI) Governance

As agents become first-class principals in the system:

- agent credentials must be managed with the same rigor as human credentials
- agent keys should have explicit expiration, rotation, and scope policies
- agent identity should be distinguishable from human identity in audit trails
- budget and rate limits should be enforceable per agent identity
- the control plane should support agent registration, capability declaration, and lifecycle management

### 19.8 Security Events

The audit system must record:

- login events
- failed auth attempts
- key creation/revocation
- policy changes
- provider secret changes
- billing adjustments
- route policy changes
- high-risk traffic anomalies (e.g., suspected prompt injections or rapid unexpected tool invocations)
- A2A delegation events and cross-agent task handoffs
- agent identity lifecycle events (registration, key rotation, deactivation)
- goal hijacking detection events
- cascading failure circuit breaker activations
- MCP session establishment and tool-auth rejection events
- realtime ephemeral-token issuance and misuse detection events

### 19.9 Administrative Security

- SSO support preferred for admin plane
- MFA required for platform admins
- IP allowlist support for private admin plane
- break-glass procedures documented


---


## 20. Compliance and Governance

The platform should be designed to support future compliance requirements even if formal certification is deferred.

### 20.1 Governance Features

- tenant-scoped audit history
- data retention controls
- region-aware storage policies
- configurable log redaction
- exportable access history
- deletion workflows
- provider resource provenance attestations
- payload-retention tier policy and access review
- delegated identity-chain visibility for high-risk actions
- protocol-specific retention controls for realtime sessions, MCP activity, and A2A task history

### 20.2 Data Residency

The routing engine and storage layer should support policy-based region constraints.

### 20.3 Compliance Hooks

Future support should be possible for:

- SOC 2 controls mapping
- ISO 27001-aligned operational controls
- enterprise access review workflows
- regulator-specific logging requirements
- EU AI Act transparency and accountability requirements for high-risk AI systems
- agent-specific governance audit trails for regulatory review
- OWASP GenAI Security controls mapping

---
