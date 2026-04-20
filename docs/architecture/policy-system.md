# 16. Policy System

[Back to Docs Index](../README.md)

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

### 16.2 Policy Enforcement Stages

Policies may be enforced at:

- request admission
- route selection
- request transformation
- response transformation
- post-response auditing
- async review workflows

### 16.3 Policy Language

The platform should expose a declarative policy format, either:

- a custom JSON/YAML policy schema compiled to internal checks, or
- an OPA/Rego-backed policy layer for advanced deployments

### 16.4 Example Policy Cases

- deny image generation for free-tier keys
- force EU routes for specific tenants
- deny specific MCP tool usage unless on the project's approved allowlist (mitigating Tool Misuse)
- terminate WebSocket sessions if token consumption exceeds the agentic session budget (mitigating Runaway Loops)
- cap max output tokens for public projects
- redact user email addresses and SSNs before logging or proxying to upstream

---
