# Roadmap Overview

[Back to Execution Index](README.md)

## Objective

Deliver the first public beta of **ku0 Trust Layer** as a Rust backend + TanStack Start frontend monorepo with:

- a live AI-resource inspection workflow for supplier endpoints
- supplier profiles, shareable reports, risk notes, and procurement evidence
- a protocol-native trusted-gateway substrate optimized for 2026 paradigms (MCP, WebRTC)
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
- The program should reuse the existing gateway/control-plane MVP, then grow toward live probing, supplier evidence, public reports, certification, enterprise monitoring, and trusted gateway routing.

## Delivery structure

The roadmap is divided into **program increments**:

- **PI-0 Foundation** - monorepo bootstrap, runtime stack, schemas, auth model, CI, and telemetry baseline
- **PI-1 Core Platform MVP** - control plane, gateway, provider resources, API keys, route receipts, and basic console flows
- **PI-2 ku0 Probe MVP** - live OpenAI-compatible endpoint inspection, redacted reports, and explicit simulated/live evidence labels
- **PI-3 Supplier Evidence Beta** - supplier profiles, shareable report pages, risk notes, supplier response, and report moderation
- **PI-4 Reports, Verified, And Monitor** - paid report intake, certification states, scheduled monitoring, billing transparency, and enterprise readiness
- **PI-5 Trusted Gateway Expansion** - production routing through inspected resources, realtime/MCP/A2A expansion where it strengthens supplier trust evidence

## Success criteria by the end of the roadmap

1. Operators can create tenants, projects, providers, routes, and API keys in the control plane.
2. Clients can send OpenAI-compatible traffic to the gateway and receive responses from at least one upstream provider.
3. Routing decisions are explainable and resilient to unhealthy upstream targets.
4. Usage is emitted from the hot path and persisted into an immutable ledger.
5. Users can run live supplier inspections and receive redacted JSON/HTML reports.
6. Supplier pages expose status, latency, error rate, risk notes, and evidence source.
7. Reports and certification states distinguish supplier-paid, ku0-tested, customer-submitted, simulated, and live evidence.
8. The repository can be built, tested, released, and observed through standard workflows.

## What makes this roadmap agent-friendly

Each task in this pack includes:

- explicit dependencies
- touched paths
- expected outputs
- acceptance criteria
- recommended ownership lane

This lets a task allocator or coding agent pick up work with minimal extra coordination.
