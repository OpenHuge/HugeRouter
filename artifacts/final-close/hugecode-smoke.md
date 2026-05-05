# Final Close HugeCode Smoke Summary

Captured: 2026-05-03 17:22 Asia/Shanghai

TR-CLOSE-01:

- PASS. Unique active listeners on `18080`, `18081`, and sandbox upstream `18082`.
- Postgres/NATS-backed control-plane, gateway, route receipt worker, and ledger worker are running.

TR-CLOSE-02:

- PASS. Route-token customer call returned HTTP `200` with `routercpt_1005`.

TR-CLOSE-03:

- PASS. Receipt, diagnostics, ledger, usage summary, and usage breakdown all returned expected runtime evidence.

TR-CLOSE-04:

- PASS. Customer API key create/list/resolve/use/revoke/fail-closed lifecycle completed.

TR-CLOSE-05:

- PASS. Billing export create/list/get/download completed and returned CSV.

TR-CLOSE-06:

- PASS. Revoked credential fail-closed and expired opening grant recovery boundaries were verified.

TR-CLOSE-07:

- PASS with sandbox payment note. Paid order fixture renewed an expired grant; unpaid order fixture stayed blocked. Real WeChat prepay is not configured locally and returned `wechat_pay_not_configured`.

Final runtime result:

- PASS.
