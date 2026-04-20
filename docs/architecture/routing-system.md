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
- session affinity constraints
- provider account health

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

6. **Selection**  
   Choose a target according to route strategy.

7. **Execution and Retry**  
   Retry according to route policy, avoiding duplicate unsafe requests.

8. **Feedback Loop**  
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
- fallback model aliases
- session stickiness behavior (e.g., tying a client-side WebRTC session to a specific upstream node)
- idempotency behavior
- shadow sampling

### 11.5 Health and Quarantine

Targets must transition through states such as:

- healthy
- degraded
- quarantined
- draining
- disabled

A target should be quarantined automatically when error thresholds, abuse thresholds, or policy thresholds are exceeded.


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
