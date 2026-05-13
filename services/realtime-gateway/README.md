# Realtime Gateway MVP

`services/realtime-gateway` now exposes a minimal but working WebSocket ingress at `GET /v1/realtime`.

## Current scope

- WebSocket-only ingress for `/v1/realtime`
- bearer-token authentication with signed ephemeral session tokens
- explicit token scope enforcement: `realtime.connect`
- explicit token expiry and optional `not_before` enforcement
- session parameter validation for `model` and `session_max_duration_seconds`
- structured lifecycle tracing for `connect`, `accepted`, `rejected`, `closed`, and `upstream-failed`
- post-handshake usage boundary only: handshake admission does **not** emit billing or metering side effects
- minimal realtime event subset:
  - server: `session.created`, `response.created`, `response.output_text.delta`, `response.completed`, `error`, `pong`
  - client: `ping`, `response.create`

## Intentionally not implemented yet

- WebRTC ingress
- ephemeral token issuance API (`POST /v1/realtime/client_secrets`)
- upstream provider session affinity or bridge to OpenAI GA realtime sessions
- audio/media frames
- tool execution and MCP/A2A session multiplexing
- billing, quota consumption, and usage flush during handshake
- durable multi-connection concurrency accounting beyond token-declared limits

These are deliberately kept out of the handshake path so later metering and upstream orchestration can evolve independently.

## Security boundary

- only signed ephemeral tokens are accepted in this MVP
- tokens must carry `realtime.connect`
- tokens are transport-bound to `websocket`
- expired or malformed tokens are rejected with `auth_invalid`
- tokens lacking the required scope are rejected with `auth_insufficient_scope`
- requested `model` or session duration outside token scope are rejected with `request_validation_failed`
- the default signing secret is development-only and should be overridden with `REALTIME_GATEWAY_SIGNING_SECRET`

## Run locally

```bash
cargo run -p realtime-gateway
```

Issue a development token:

```bash
cargo run -p protocol-realtime --example issue_token -- \
  dev-only-signing-secret-change-me browser-client tenant-dev project-dev realtime-default 300
```

Then connect a WebSocket client to:

```text
ws://127.0.0.1:8081/v1/realtime?model=realtime-default
Authorization: Bearer <issued-token>
```
