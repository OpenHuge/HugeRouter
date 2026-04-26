# Future Relay Capabilities

HugeRouter relay should evolve from an OpenAI-compatible ingress into a
protocol-neutral control and data plane for model, tool, agent, realtime, and
commerce traffic. The implementation should reduce operational and account
risk through explicit policy gates, attribution, quotas, and audit trails rather
than attempting to bypass provider enforcement.

## Capability Model

The gateway now exposes an internal capability surface at
`GET /internal/relay/capabilities`. It reports:

- Current adapter-backed protocol families and lifecycle families.
- Planned protocol surfaces for MCP, A2A, and realtime WebRTC.
- Routing controls such as capability-aware routing, health filtering,
  retryable fallback, transit-loop prevention, and budget projection.
- Safety controls such as API-key scope resolution, provider credential
  isolation, tenant/project authorization, normalized error boundaries, route
  receipts, and an explicit terms/policy gate for external relays.
- Observability and commerce controls for route receipts, provider attempts,
  fallback transitions, usage events, billable cost projection, and merchant
  replay evidence.

## Near-Term Implementation Order

1. **Protocol capability registry** — Keep adapter manifests and relay
   capabilities explicit so routing can choose by protocol, lifecycle family,
   streaming support, and safety controls.
2. **MCP relay gate** — Add tool/resource/prompt registration only behind
   allowlists, consent receipts, tenant-scoped resources, and tool-call audit
   records.
3. **A2A relay gate** — Add agent-card discovery, delegation policy, task
   lifecycle tracing, and cross-agent identity boundaries before accepting
   delegated work.
4. **Realtime relay controls** — Add session budgets, interruption traces,
   media redaction policy, and partial usage metering before exposing long-lived
   realtime sessions.
5. **Provider-risk governance** — Enforce credential isolation, request
   attribution, provider-specific rate limits, and terms-of-service policy gates
   for every external relay target.

## Reference Material

The local OpenAI Codex source checkout used as an implementation reference is
at `/Users/han/Documents/openai-codex`. Treat it as a reference for CLI agent
workflow patterns, review loops, and safe local orchestration only; relay
provider compliance must remain governed by HugeRouter policies and provider
terms.
