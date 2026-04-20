# 22. Public Gateway API

[Back to Docs Index](../README.md)

The public gateway API is the request ingress for customer workloads.

### 22.1 Public API Principles

- protocol compatibility where declared
- explicit `ws://` and `wss://` ingress for bidirectional streaming and Realtime APIs
- request tracing headers
- streaming support (both HTTP chunked and persistent WebSockets)
- explicit error normalization
- stable auth and rate limit semantics

### 22.2 Public API Features

- logical model aliasing
- protocol-native request acceptance
- tenant/project isolation
- streaming and non-streaming modes
- **Bidirectional WebSocket Proxying:** Maintain stateful connections, apply in-flight frame inspection, and proxy tool execution contexts (e.g., via MCP) without breaking the stream.
- usage and request IDs in response metadata where compatible
- progressive metering events emitted periodically during long-lived `ws` sessions
- optional debug headers in approved environments

### 22.3 Supported Ingress Endpoints

1. **REST / HTTP Streaming:** `https://api.gateway.local/v1/chat/completions` (OpenAI compatible)
2. **WebSocket / Realtime:** `wss://api.gateway.local/v1/realtime` (for voice/audio and continuous agent interactions)
3. **Ephemeral Token Generation:** `POST /v1/realtime/client_secrets` (to allow secure browser WebRTC/WS connections without exposing master keys)

### 22.3 Debug Modes

A controlled debug mode may expose:

- selected route target
- retry count
- route policy ID
- upstream provider class

This must be disabled by default for public production traffic.

---
