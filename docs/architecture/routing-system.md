# 11. Routing System

[Back to Docs Index](../README.md)

Routing is the core differentiator of the platform.

### 11.1 Current Implementation Reality

The repository should currently be described this way:

- `implemented`: one synchronous OpenAI-compatible HTTP path with bearer-auth validation, active snapshot loading, basic candidate ranking, and retry to the next eligible target on retryable failures
- `bootstrap-only`: route simulation exists, but it is not yet authoritative parity with the hot-path execution logic
- `planned`: streaming, realtime, MCP/A2A routing, budget admission, quota admission, concurrency admission, durable route-receipt persistence, and budget or rate-limit response headers

The rest of this document defines the target contract. It should not be read as a claim that every behavior below already exists in the running gateway.

### 11.2 Routing Inputs

The routing engine should consider:

- requested model alias
- required capabilities
- optional capabilities
- tenant-level policy
- project-level policy
- credential scope
- target provider availability
- active health score
- historical latency percentiles
- historical error rates
- unit economics
- region restrictions
- compliance restrictions
- rate limit state
- provider-native quota window headroom
- pre-admission budget headroom
- target provenance class
- ingress or regional failure-domain health
- session affinity constraints
- provider account health
- semantic cache hit eligibility and similarity score
- A2A agent card capability matching
- agent identity and delegation chain context
- protocol family and lifecycle shape, such as stateless request, realtime session, MCP tool interaction, or A2A task continuation

### 11.3 Route Selection Stages

1. **Candidate Expansion**  
   Expand a logical model alias into possible route targets.

2. **Capability Filtering**  
   Remove candidates that cannot satisfy required capabilities.

3. **Policy Filtering**  
   Remove candidates disallowed by tenant, region, compliance, provenance, or delegated-authority rules.

4. **Health Filtering**  
   Remove unhealthy or quarantine-state candidates.

5. **Guardrail Filtering**
   Evaluate route-level guardrails such as semantic-cache bypass, memory restrictions, MCP tool restrictions, or A2A chain-depth limits before ranking proceeds.

6. **Scoring**
   Score remaining candidates based on weighted criteria.

7. **Selection**  
   Choose a target according to the declared route strategy.

8. **Admission Control**
   Confirm budget, quota, concurrency, and trust-class eligibility before committing to upstream execution.

9. **Execution and Retry**
   Retry according to route policy, avoiding duplicate unsafe requests and recording every fallback transition.

10. **Route Receipt**
    Persist or emit a structured route decision record for diagnostics, support, billing explanation, and simulation parity.

11. **Feedback Loop**
    Persist outcome signals for future scoring.

Stage-ordering rule:

- if a later stage changes the effective target, side-effect class, or delegated authority, the affected policy and admission stages must be re-run and the transition must appear in the route receipt

### 11.4 Route Strategy Families

Supported named strategies should include:

- `priority`: choose the highest-ranked eligible target by explicit operator-managed priority
- `weighted`: distribute requests by configured weights after eligibility filtering
- `latency-aware`: optimize for recent latency while respecting capability and policy floors
- `cost-aware`: optimize for estimated marginal cost while respecting quality and trust constraints
- `least-busy`: prefer the target with the healthiest current concurrency or queue pressure
- `quality-floor-with-budget`: choose the lowest-cost target that still satisfies a declared minimum quality or capability floor
- `sticky-session`: pin long-lived sessions to a selected target with explicit rebinding rules
- `reconnect-affinity`: prefer reconnecting a dropped `ws` or realtime session to the same upstream execution locus
- `canary`: send a bounded percentage to a candidate set for evaluation without redefining the primary strategy
- `shadow`: duplicate traffic for evaluation only when the operation class is safe for duplicate execution
- `semantic-cache-first`: check cache before upstream dispatch only when policy says replay is safe
- `agent-capability-match`: route delegated A2A work to the best matching verified agent capability set
- `workflow-aware`: route multi-step delegated execution with shared budget and chain context

Default policy:

- semantic-cache-first routing should be disabled by default for stateful realtime sessions, terminal A2A task transitions, and side-effecting MCP tool operations unless a narrower policy explicitly allows it

For every strategy, the control plane should be able to explain:

- the declared strategy family
- the candidate set it operated on
- the ranking or tie-break inputs it used
- the fallback order that will apply if execution fails

### 11.5 Route Policies

A route policy should define:

- match criteria
- allowed providers
- denied providers
- preferred providers
- preferred regions
- retry count
- retry backoff
- timeout budget
- max estimated marginal cost
- fallback model aliases
- session stickiness behavior (e.g., tying a client-side WebRTC session to a specific upstream node)
- idempotency behavior
- shadow sampling
- allowed provenance classes
- quota reserve strategy
- time-to-first-token budget
- semantic cache policy (eligible, bypass, or invalidate)
- A2A delegation scope and allowed agent targets
- route explanation sampling or retention tier
- protocol-bridge policy, such as whether MCP-to-A2A or A2A-to-MCP bridging is allowed for a route family

### 11.6 Health and Quarantine

Targets must transition through states such as:

- healthy
- degraded
- quarantined
- draining
- disabled

A target should be quarantined automatically when error thresholds, abuse thresholds, or policy thresholds are exceeded.

Health must be tracked across separate failure domains rather than one flattened score. At minimum:

- provider API health
- credential or account-pool health
- regional ingress health
- protocol-path health (e.g. batch HTTP vs streaming vs realtime)
- gateway deployment health

### 11.7 Admission Control and Rate-Window Modeling

Routing must treat rate limits, quotas, and budgets as first-class admission control inputs.

The system should support:

- provider-native RPM and TPM windows, including sub-minute windows where upstreams enforce them
- hierarchical limits across platform, tenant, project, credential, and provider resource
- concurrency caps for long-lived streaming or realtime sessions
- task-budget and step-budget controls for delegated A2A workflows
- pre-admission reserve accounting for estimated request cost
- hard-stop and soft-throttle modes with explicit policy control

Admission should converge on a stable terminal vocabulary:

- `admitted`
- `rejected_no_candidate`
- `rejected_budget`
- `rejected_quota`
- `rejected_concurrency`
- `rejected_policy`
- `rejected_trust_class`

When a request is rejected before execution, the rejection should still emit a route decision record and policy reason so that customers can distinguish "policy denied" from "provider failed".

### 11.8 Route Receipt and Explainability

Every routed request should emit a compact route receipt containing:

- receipt identifier
- request identifier and trace identifier
- protocol family and requested model alias
- effective tenant, project, credential, and config snapshot identifiers
- request classification inputs used by the routing engine
- route strategy family
- candidate list with exclusion reasons
- score contributions by dimension such as latency, cost, health, trust, and residency
- selected target and policy version
- admission terminal state
- normalized policy decision summary or reason code set
- quota or budget reservation outcome
- fallback chain traversal, if any
- final execution outcome category

Minimal route receipt shape:

```text
RouteReceipt
- route_receipt_id
- request_id
- trace_id
- snapshot_id
- protocol_family
- requested_model_alias
- strategy_family
- candidates[]
- selected_target
- admission_result
- policy_summary
- budget_reservation
- fallback_transitions[]
- terminal_outcome
```

This receipt is the canonical artifact for support tooling, customer spend explanations, route simulation parity, and incident review.

### 11.9 Provenance and Trust-Class Filtering

The router should refuse to treat all upstream capacity as interchangeable.

Route candidates should carry provenance metadata such as:

- official API
- official managed gateway
- BYO customer credential
- dedicated managed account
- shared brokered account pool
- unofficial or reverse-engineered client channel

Policy should be able to ban or degrade unsafe trust classes by environment, tenant tier, or compliance mode. Reverse-engineered or opaque brokered channels should be disabled by default for enterprise operation.

---

## 37. Route Simulation Feature

A route simulation feature should exist in both backend and frontend.

### 37.1 Purpose

Allow operators to test how a hypothetical request would route without sending traffic upstream.

### 37.2 Inputs

- tenant
- project
- credential scope
- protocol family
- model alias
- required capabilities
- region
- expected traffic class

### 37.3 Outputs

- eligible candidates
- filtered-out candidates with reasons
- final ranked order
- selected target
- estimated cost and latency bands
- simulated admission result
- predicted fallback chain
- effective snapshot identifier used by the simulation

### 37.4 Simulation Status Rule

Current repository status should be described as:

- `bootstrap-only`: a simulation surface exists, but it is not yet guaranteed to share the same code path or decision parity as live routing

Target contract:

- route simulation should become authoritative only when it consumes the same normalized inputs, snapshot shape, strategy family definitions, and admission vocabulary as the hot path

This feature is highly valuable for debugging route policy complexity.

---
