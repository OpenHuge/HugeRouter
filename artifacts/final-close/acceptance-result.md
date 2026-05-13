# Final Close Acceptance Result

Captured: 2026-05-03 17:22 Asia/Shanghai

Verdict: PASS

Scope:

- Active runtime was rebuilt and restarted from branch `lab`.
- Control-plane is running in Postgres mode.
- Gateway is using NATS and active control-plane config.
- Route receipt and ledger workers are running.
- Provider calls were made against a local OpenAI-compatible sandbox upstream on `127.0.0.1:18082`.

Runtime evidence:

- Route token call: HTTP `200`, receipt `routercpt_1005`.
- Receipt lookup: HTTP `200`.
- Diagnostics lookup: HTTP `200`, provider status `success`.
- Ledger entry exists for `routercpt_1005`.
- Usage summary/breakdown: HTTP `200`.
- API key lifecycle: create/list/resolve/use/revoke/fail-closed completed.
- Billing export: create/list/get/download completed.
- Renewal: paid fixture renewed expired grant; unpaid fixture remained blocked.

Code fix included:

- `services/control-plane-api/src/store.rs` casts Postgres aggregate `SUM(bigint)` values to `BIGINT` in usage summary and breakdown SQL to prevent runtime `NUMERIC` decode panic.

Verification:

- `control-plane-api`: 67 tests passed.
- `gateway-api route_token`: 6 tests passed.
- `gateway-api events_protocol`: 12 tests passed.
- `console-web`: 39 tests passed.
- `console-web typecheck`: passed.
- `git diff --check`: passed with LF/CRLF warnings only.

Environment note:

- Real WeChat prepay was not attempted because this local runtime does not have merchant private-key configuration. The prepay route exists and returns `wechat_pay_not_configured`; renewal semantics were verified through sandbox/manual order fixtures as planned.
