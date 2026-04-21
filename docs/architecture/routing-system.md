# 11. Routing System

[Back to Docs Index](../README.md)

Routing is the core differentiator of the platform.

### 11.1 Routing Inputs

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

### 11.2 Route Selection Stages

1. **Candidate Expansion**  
   Expand a logical model alias into possible route targets.

2. **Capability Filtering**  
   Remove candidates that cannot satisfy required capabilities.

3. **Policy Filtering**  
   Remove candidates disallowed by tenant, region, or compliance rules.

4. **Health Filtering**  
   Remove unhealthy or quarantine-state candidates.

5. **Scoring**  
   Score remaining candidates based on weighted criteria.

6. **Admission Control**  
   Confirm budget, quota, concurrency, and trust-class eligibility before committing to a target.

7. **Selection**  
   Choose a target according to route strategy.

8. **Execution and Retry**  
   Retry according to route policy, avoiding duplicate unsafe requests.

9. **Route Receipt**  
   Persist a structured route decision record for diagnostics, support, billing explanation, and simulation parity.

10. **Feedback Loop**  
   Persist outcome signals for future scoring.

### 11.3 Route Strategies

Supported strategies should include:

- lowest cost within policy
- lowest latency within policy
- weighted score blend
- sticky session routing (critical for long-lived WebSocket / WebRTC agentic contexts)
- stateful reconnection affinity (resuming dropped `ws` connections on the same adapter)
- premium-first with downgrade fallback
- region-preferred fallback region
- canary percentage routing
- shadow traffic for evaluation
- intent- or class-aware routing with confidence thresholds and deterministic fallback behavior
- budget-aware quality floor selection, where the cheapest target must still satisfy tenant-defined quality or capability minima
- semantic cache-first routing, where cache-eligible requests check for semantically similar cached responses before upstream dispatch
- A2A agent delegation routing, where tasks are routed to the best-matching agent based on Agent Card capability declarations
- multi-agent workflow-aware routing, where sequential agent steps in a delegation chain are routed with shared context and budget awareness

Default policy:

- semantic-cache-first routing should be disabled by default for stateful realtime sessions, terminal A2A task transitions, and side-effecting MCP tool operations unless a narrower policy explicitly allows it

### 11.4 Route Policies

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

### 11.5 Health and Quarantine

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

### 11.6 Admission Control and Rate-Window Modeling

Routing must treat rate limits, quotas, and budgets as first-class admission control inputs.

The system should support:

- provider-native RPM and TPM windows, including sub-minute windows where upstreams enforce them
- hierarchical limits across platform, tenant, project, credential, and provider resource
- concurrency caps for long-lived streaming or realtime sessions
- task-budget and step-budget controls for delegated A2A workflows
- pre-admission reserve accounting for estimated request cost
- hard-stop and soft-throttle modes with explicit policy control

When a request is rejected before execution, the rejection should still emit a route decision record and policy reason so that customers can distinguish "policy denied" from "provider failed".

### 11.7 Route Receipt and Explainability

Every routed request should emit a compact route receipt containing:

- request classification inputs used by the routing engine
- candidate list with exclusion reasons
- score contributions by dimension such as latency, cost, health, trust, and residency
- selected target and policy version
- quota or budget reservation outcome
- fallback chain traversal, if any

This receipt is the canonical artifact for support tooling, customer spend explanations, route simulation parity, and incident review.

### 11.8 Provenance and Trust-Class Filtering

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

This feature is highly valuable for debugging route policy complexity.

---
