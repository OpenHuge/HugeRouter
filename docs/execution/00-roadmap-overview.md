# Roadmap Overview

[Back to Execution Index](README.md)

## Objective

Deliver the first production-capable version of **AI Traffic OS** as a Rust backend + TanStack Start frontend monorepo with:

- a protocol-native gateway optimized for 2026 paradigms (MCP, WebRTC)
- a typed control plane
- Turbo-managed frontend and shared-package workflows
- a HeroUI-based console design system
- immutable usage and ledger handling
- intelligent routing, semantic caching, and reliability controls
- first-class security, observability, and release automation

## Planning assumptions

- The repository starts from the split Markdown specification already present in this monorepo.
- Multiple agents can work in parallel when tasks have distinct file ownership or only depend on stable interfaces.
- Database, event bus, and generated schema workflows must be available early because they unlock most other streams.
- The program should produce a usable gateway MVP quickly, then grow toward enterprise reliability and billing maturity.

## Delivery structure

The roadmap is divided into **program increments**:

- **PI-0 Foundation** - monorepo bootstrap, runtime stack, schemas, auth model, CI, and telemetry baseline
- **PI-1 Core Platform MVP** - control plane skeleton, OpenAI-compatible gateway, one provider adapter, static routing, API keys, and basic console flows
- **PI-2 Multi-Protocol and Semantic Edge** - Anthropic/Gemini support, semantic caching, MCP-aware IR, route health, route diagnostics, audit pipeline
- **PI-3 Billing and Enterprise Hardening** - immutable ledger, pricing engine, usage dashboards, SSO, load testing
- **PI-4 Ecosystem and Agentic Routing** - realtime WebRTC gateway with ephemeral keys, A2A routing, gateway-of-gateways adapters, advanced policy

## Success criteria by the end of the roadmap

1. Operators can create tenants, projects, providers, routes, and API keys in the control plane.
2. Clients can send OpenAI-compatible traffic to the gateway and receive responses from at least one upstream provider.
3. Routing decisions are explainable and resilient to unhealthy upstream targets.
4. Usage is emitted from the hot path and persisted into an immutable ledger.
5. The console exposes enough diagnostics and billing state for real operation.
6. The repository can be built, tested, released, and observed through standard workflows.

## What makes this roadmap agent-friendly

Each task in this pack includes:

- explicit dependencies
- touched paths
- expected outputs
- acceptance criteria
- recommended ownership lane

This lets a task allocator or coding agent pick up work with minimal extra coordination.
