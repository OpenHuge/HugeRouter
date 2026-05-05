# Final Close Customer Call Probe

Captured: 2026-05-03 17:12 Asia/Shanghai

Route token issue:

- `POST http://127.0.0.1:18080/v1/hugerouter/route-tokens`
- scopes: `route:codex`, `provider:hugerouter-commercial`
- result: HTTP `200`
- token id: `hgrtkn_adb8286067be0737`

Chat completion probe:

- `POST http://127.0.0.1:18080/v1/chat/completions`
- model: `reasoning-fast`
- auth: issued route token
- result: HTTP `200`
- content: `final-close mock ok`
- `x-request-id`: `req_1005`
- `x-trace-id`: `trace_1005`
- `x-route-receipt-id`: `routercpt_1005`
- `x-config-snapshot-id`: `cfgsnap_gateway_v1`
- `x-debug-selected-target`: `prvrsrc_openai_primary`

Interpretation:

- TR-CLOSE-02 passes in active runtime against the sandbox upstream.
- No production provider call was used for this probe.
