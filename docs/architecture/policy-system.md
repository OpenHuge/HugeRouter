# 16. Policy System

[Back to Docs Index](../README.md)

### 16.0 Current Implementation Reality

The repository should currently be described this way:

- `implemented`: basic auth and route-adjacent checks exist in the gateway and control plane, but not as a full normalized policy engine
- `bootstrap-only`: some control-plane auth and workspace behavior is suitable for bootstrap environments rather than production policy enforcement
- `planned`: normalized policy reason codes, approval checkpoints, quota and budget enforcement, MCP and A2A governance, and route-receipt policy summaries

### 16.1 Policy Categories

Policies should exist for:

- authentication
- authorization
- regional routing
- data residency
- content filtering (including real-time prompt injection detection)
- tool and MCP governance (allowlists and scope limits)
- model allow/deny lists
- hierarchical quotas (team > project > session) to prevent Denial of Wallet
- maximum spend per time window
- request shape restrictions
- PII detection and redaction
- audit retention
- shadow routing permissions
- A2A agent delegation governance (allowed agents, task scope limits, delegation depth)
- Agent Card validation and trust requirements
- non-human identity (NHI) scope and budget restrictions
- semantic cache eligibility and invalidation rules
- agent memory and context integrity validation
- browser and external tool side-effect governance
- human approval checkpoints and escalation rules
- third-party system egress and delegated credential handling

### 16.1.1 Policy Target Surfaces

Policies should be attachable to more than just a request envelope. The target surfaces should remain explicit:

- identity: principal, credential, delegation chain
- request: one admitted ingress unit
- session: long-lived realtime, MCP, or browser-assisted execution context
- task: durable delegated work item
- operation: one inference, tool, browser, memory, or delegation step
- resource: provider resource, MCP server, agent endpoint, browser profile, or memory system

### 16.1.2 Canonical Policy Effects

The policy layer should converge on a small set of outcomes that every protocol family can map to:

- `allow`
- `deny`
- `require_approval`
- `degrade`
- `isolate`
- `observe_only`

`require_approval` is a normal control outcome, not an exceptional escape hatch. Recent agent runtimes normalize pause-and-resume flows, so HugeRouter should treat approval checkpoints as first-class governance results.

### 16.2 Policy Enforcement Stages

Policies may be enforced at:

- request admission
- session setup
- route selection
- request transformation
- pre-operation execution
- response transformation
- post-operation reconciliation
- post-response auditing
- async review workflows

Recommended default ordering:

1. identity and delegation validation
2. request and session admission
3. route and target eligibility
4. operation-specific policy
5. post-execution audit and retention policy

If a later stage changes the effective target, side-effect class, or delegated authority, the impacted policies should be re-evaluated and the transition should be recorded.

### 16.3 Policy Language

The platform should expose a declarative policy format, either:

- a custom JSON/YAML policy schema compiled to internal checks, or
- an OPA/Rego-backed policy layer for advanced deployments

Regardless of authoring format, the policy engine should emit stable machine-readable reason codes and one normalized decision summary that can be embedded into route receipts, audit events, approval checkpoints, and replay capsules.

### 16.3.1 Decision Summary Shape

Each material policy decision should be explainable with at least:

- `policy_effect`
- `reason_code`
- `policy_scope`
- `matched_rule_ids`
- `requires_approval` boolean
- `snapshot_id`

### 16.3.2 Normalized Policy Reason Vocabulary

Policy enforcement should use a small stable reason-code vocabulary that other systems can rely on.

At minimum, the platform should reserve machine-readable codes for:

- `identity_invalid`
- `session_not_allowed`
- `route_not_allowed`
- `resource_not_allowed`
- `budget_exceeded`
- `quota_exceeded`
- `concurrency_exceeded`
- `trust_class_denied`
- `approval_required`
- `residency_denied`
- `tool_not_allowed`
- `delegation_denied`
- `memory_write_denied`

These codes should be safe to embed into route receipts, error envelopes, audit events, support tooling, and billing explanations.

### 16.4 High-Side-Effect Operation Governance

Recent open source agent systems have made browser automation, custom tools, and long-running delegated actions common rather than exotic. The gateway should therefore classify operation risk explicitly instead of pretending every tool call is equivalent to a read-only inference.

Recommended baseline:

- `tool_read` and `memory_recall` may default to ordinary admission plus allowlist checks
- `tool_write`, `browser_write`, `memory_retain`, and `delegation_submit` should default to stricter scope, retention, and approval evaluation
- any operation that can send data to a third-party system, reuse browser-authenticated state, or mutate an external system should be eligible for `require_approval` or `isolate`

### 16.5 Memory and Semantic Cache Boundary

Policy must distinguish between:

- semantic cache reuse
- memory recall from an external or application-owned memory system
- memory retain or reflect operations that create new durable observations

Recommended invariants:

- semantic cache eligibility should never imply permission to read or write agent memory
- memory-retain operations should carry explicit retention and data-class policy
- replay and audit artifacts should preserve that a memory-bearing operation occurred without forcing raw memory content retention

### 16.6 Delegation and Third-Party System Boundary

Policy should assume that delegated execution may cross trust, geography, and retention boundaries.

Recommended defaults:

- every delegation attempt should evaluate delegating principal, delegated subject, target agent, and maximum chain depth
- every third-party system hop should remain visible as a policy target, not hidden behind a generic adapter success path
- credentials, browser profiles, session cookies, or upstream auth material reused for agent execution should be treated as high-sensitivity delegated resources

### 16.7 Snapshotting and Explainability

Policy decisions that materially affect admission, routing, approval, retention, or delegation should be attributable to one effective snapshot and one stable reason vocabulary.

This is required so operators can answer:

- why a request or task was denied
- why approval was required
- why a browser or tool action was isolated
- why semantic cache was bypassed
- why a delegated action could or could not cross a trust boundary

### 16.7.1 Route and Admission Boundary

The policy layer should remain explicit about which decisions are policy decisions and which are routing or admission decisions.

Boundary rule:

- policy decides whether a request, session, task, operation, or resource is allowed, denied, degraded, isolated, or paused for approval
- routing decides which eligible target is preferred
- admission decides whether budget, quota, concurrency, and trust-class state still allow execution on the selected target

Cross-system rule:

- the route receipt should embed the normalized policy decision summary plus the admission terminal state
- if a policy result forces target re-selection or changes operation class, routing and admission must be re-evaluated
- policy, routing, and billing surfaces must reuse the same reason codes where the customer-visible meaning is the same

### 16.8 Example Policy Cases

- deny image generation for free-tier keys
- force EU routes for specific tenants
- deny specific MCP tool usage unless on the project's approved allowlist (mitigating Tool Misuse)
- terminate WebSocket sessions if token consumption exceeds the agentic session budget (mitigating Runaway Loops)
- cap max output tokens for public projects
- redact user email addresses and SSNs before logging or proxying to upstream
- reject A2A task delegation from unverified agents lacking valid Agent Cards
- limit A2A delegation chain depth to prevent cascading agent loops
- enforce per-agent-identity budget ceilings independent of tenant budgets
- bypass semantic cache for requests marked as requiring fresh upstream responses
- require approval before `browser_write` operations that can submit forms, purchase resources, or mutate external state
- deny `memory_retain` when the request classification forbids durable storage of customer content
- isolate or deny third-party tool execution when delegated credentials or browser-authenticated sessions would leave the approved residency or trust boundary

---
